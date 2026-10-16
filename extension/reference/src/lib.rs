//! SurveyCraft's portable CADCraft guest component.

// `wit-bindgen` generates the low-level unsafe component ABI. SurveyCraft's handwritten code stays
// safe, and this exception is deliberately confined to this private bindings module.
#[allow(unsafe_code)]
mod component {
    wit_bindgen::generate!({
        path: "../../crates/cadcraft/wit",
        world: "cadcraft-extension",
    });

    use cadcraft::extension::drawing;
    use cadcraft::extension::geometry::{
        LineRequest as WitLineRequest, Point3 as WitPoint3, PointRequest as WitPointRequest,
        TextRequest as WitTextRequest,
    };
    use surveycraft_cadcraft::{
        CadcraftHost, EntityId, HostError, InsertPointsRequest, LineRequest, PointRequest,
        TextRequest, TransactionId, insert_points, run_reference_extension,
    };
    use surveycraft_geometry::Point3;

    struct SurveyCraftExtension;

    impl Guest for SurveyCraftExtension {
        fn run_command(command: String, parameters_json: String) -> Result<String, String> {
            let mut host = ComponentHost;
            match command.as_str() {
                "insert-points" => {
                    let request: InsertPointsRequest = serde_json::from_str(&parameters_json)
                        .map_err(|error| format!("invalid point table: {error}"))?;
                    let result =
                        insert_points(&mut host, &request).map_err(|error| error.to_string())?;
                    serde_json::to_string(&result)
                        .map_err(|error| format!("cannot encode insertion result: {error}"))
                }
                "reference-line" => {
                    let request = LineRequest {
                        start: Point3::new(0.0, 0.0, 0.0).map_err(|error| error.to_string())?,
                        end: Point3::new(10.0, 0.0, 0.0).map_err(|error| error.to_string())?,
                        layer: "SURVEY_REFERENCE".to_owned(),
                    };
                    let entity = run_reference_extension(&mut host, &request)
                        .map_err(|error| error.to_string())?;
                    Ok(format!(r#"{{"entityId":{}}}"#, entity.0))
                }
                _ => Err(format!("unknown SurveyCraft command route '{command}'")),
            }
        }
    }

    struct ComponentHost;

    impl CadcraftHost for ComponentHost {
        fn begin_transaction(&mut self, label: &str) -> Result<TransactionId, HostError> {
            drawing::begin_transaction(label)
                .map(TransactionId)
                .map_err(host_error)
        }

        fn create_line(
            &mut self,
            transaction: TransactionId,
            request: &LineRequest,
        ) -> Result<EntityId, HostError> {
            let request = WitLineRequest {
                start: wit_point(request.start),
                end: wit_point(request.end),
                layer: request.layer.clone(),
            };
            drawing::create_line(transaction.0, &request)
                .map(EntityId)
                .map_err(host_error)
        }

        fn create_point(
            &mut self,
            transaction: TransactionId,
            request: &PointRequest,
        ) -> Result<EntityId, HostError> {
            let request = WitPointRequest {
                position: wit_point(request.position),
                layer: request.layer.clone(),
            };
            drawing::create_point(transaction.0, &request)
                .map(EntityId)
                .map_err(host_error)
        }

        fn create_text(
            &mut self,
            transaction: TransactionId,
            request: &TextRequest,
        ) -> Result<EntityId, HostError> {
            let request = WitTextRequest {
                position: wit_point(request.position),
                value: request.value.clone(),
                height: request.height,
                rotation_degrees: request.rotation_degrees,
                layer: request.layer.clone(),
            };
            drawing::create_text(transaction.0, &request)
                .map(EntityId)
                .map_err(host_error)
        }

        fn commit_transaction(&mut self, transaction: TransactionId) -> Result<(), HostError> {
            drawing::commit_transaction(transaction.0).map_err(host_error)
        }

        fn rollback_transaction(&mut self, transaction: TransactionId) -> Result<(), HostError> {
            drawing::rollback_transaction(transaction.0).map_err(host_error)
        }
    }

    fn wit_point(point: Point3) -> WitPoint3 {
        WitPoint3 {
            x: point.x(),
            y: point.y(),
            z: point.z(),
        }
    }

    fn host_error(error: drawing::HostError) -> HostError {
        HostError::new(format!("{error:?}"))
    }

    export!(SurveyCraftExtension with_types_in self);
}
