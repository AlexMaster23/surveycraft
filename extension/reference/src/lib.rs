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
        ArcRequest as WitArcRequest, CircleRequest as WitCircleRequest,
        EntityAppearance as WitEntityAppearance, EntityKind as WitEntityKind,
        EntityView as WitEntityView, LineRequest as WitLineRequest, Point3 as WitPoint3,
        PointRequest as WitPointRequest, SolidFillRequest as WitSolidFillRequest,
        TextRequest as WitTextRequest,
    };
    use surveycraft_cadcraft::{
        ArcRequest, CadcraftHost, CircleRequest, CodeBookManagerRequest, EntityAppearance,
        EntityId, EntityKind, EntityView, GenerateCodedFeaturesRequest, HostError,
        InsertPointsRequest, LineRequest, PointManagerRequest, PointRequest, SolidFillRequest,
        TextRequest, TransactionId, generate_coded_features, insert_points, inspect_project,
        manage_code_book, manage_points, run_reference_extension,
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
                "inspect-project" => {
                    let result = inspect_project(&mut host).map_err(|error| error.to_string())?;
                    serde_json::to_string(&result)
                        .map_err(|error| format!("cannot encode inspection result: {error}"))
                }
                "manage-points" => {
                    let request: PointManagerRequest = serde_json::from_str(&parameters_json)
                        .map_err(|error| format!("invalid point manager action: {error}"))?;
                    let result =
                        manage_points(&mut host, &request).map_err(|error| error.to_string())?;
                    serde_json::to_string(&result)
                        .map_err(|error| format!("cannot encode point manager result: {error}"))
                }
                "manage-code-book" => {
                    let request: CodeBookManagerRequest = serde_json::from_str(&parameters_json)
                        .map_err(|error| format!("invalid code-book manager action: {error}"))?;
                    let result =
                        manage_code_book(&mut host, &request).map_err(|error| error.to_string())?;
                    serde_json::to_string(&result)
                        .map_err(|error| format!("cannot encode code-book manager result: {error}"))
                }
                "generate-coded-features" => {
                    let request = if parameters_json.trim() == "null" {
                        GenerateCodedFeaturesRequest::default()
                    } else {
                        serde_json::from_str(&parameters_json)
                            .map_err(|error| format!("invalid coded-feature options: {error}"))?
                    };
                    let result = generate_coded_features(&mut host, request)
                        .map_err(|error| error.to_string())?;
                    serde_json::to_string(&result)
                        .map_err(|error| format!("cannot encode coded-feature result: {error}"))
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

        fn create_arc(
            &mut self,
            transaction: TransactionId,
            request: &ArcRequest,
        ) -> Result<EntityId, HostError> {
            let request = WitArcRequest {
                center: wit_point(request.center),
                radius: request.radius,
                start_degrees: request.start_degrees,
                end_degrees: request.end_degrees,
                layer: request.layer.clone(),
            };
            drawing::create_arc(transaction.0, &request)
                .map(EntityId)
                .map_err(host_error)
        }

        fn create_circle(
            &mut self,
            transaction: TransactionId,
            request: &CircleRequest,
        ) -> Result<EntityId, HostError> {
            let request = WitCircleRequest {
                center: wit_point(request.center),
                radius: request.radius,
                layer: request.layer.clone(),
            };
            drawing::create_circle(transaction.0, &request)
                .map(EntityId)
                .map_err(host_error)
        }

        fn create_solid_fill(
            &mut self,
            transaction: TransactionId,
            request: &SolidFillRequest,
        ) -> Result<EntityId, HostError> {
            let request = WitSolidFillRequest {
                boundary: request.boundary.iter().copied().map(wit_point).collect(),
                layer: request.layer.clone(),
                color: request.color.clone(),
            };
            drawing::create_solid_fill(transaction.0, &request)
                .map(EntityId)
                .map_err(host_error)
        }

        fn set_entity_appearance(
            &mut self,
            transaction: TransactionId,
            entity: EntityId,
            appearance: &EntityAppearance,
        ) -> Result<(), HostError> {
            let appearance = WitEntityAppearance {
                color: appearance.color.clone(),
                lineweight_mm: appearance.lineweight_mm,
            };
            drawing::set_entity_appearance(transaction.0, entity.0, &appearance).map_err(host_error)
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

        fn get_drawing_metadata(&mut self, key: &str) -> Result<Option<String>, HostError> {
            drawing::get_drawing_metadata(key).map_err(host_error)
        }

        fn set_drawing_metadata(
            &mut self,
            transaction: TransactionId,
            key: &str,
            value: &str,
        ) -> Result<(), HostError> {
            drawing::set_drawing_metadata(transaction.0, key, value).map_err(host_error)
        }

        fn remove_drawing_metadata(
            &mut self,
            transaction: TransactionId,
            key: &str,
        ) -> Result<(), HostError> {
            drawing::remove_drawing_metadata(transaction.0, key).map_err(host_error)
        }

        fn get_entity(&mut self, entity: EntityId) -> Result<Option<EntityView>, HostError> {
            drawing::get_entity(entity.0)
                .map_err(host_error)?
                .map(entity_view)
                .transpose()
        }

        fn get_entity_metadata(
            &mut self,
            entity: EntityId,
            key: &str,
        ) -> Result<Option<String>, HostError> {
            drawing::get_entity_metadata(entity.0, key).map_err(host_error)
        }

        fn find_entities_by_metadata(
            &mut self,
            key: &str,
            value: &str,
        ) -> Result<Vec<EntityView>, HostError> {
            drawing::find_entities_by_metadata(key, value)
                .map_err(host_error)?
                .into_iter()
                .map(entity_view)
                .collect()
        }

        fn set_entity_metadata(
            &mut self,
            transaction: TransactionId,
            entity: EntityId,
            key: &str,
            value: &str,
        ) -> Result<(), HostError> {
            drawing::set_entity_metadata(transaction.0, entity.0, key, value).map_err(host_error)
        }

        fn remove_entity_metadata(
            &mut self,
            transaction: TransactionId,
            entity: EntityId,
            key: &str,
        ) -> Result<(), HostError> {
            drawing::remove_entity_metadata(transaction.0, entity.0, key).map_err(host_error)
        }

        fn update_point(
            &mut self,
            transaction: TransactionId,
            entity: EntityId,
            request: &PointRequest,
        ) -> Result<(), HostError> {
            let request = WitPointRequest {
                position: wit_point(request.position),
                layer: request.layer.clone(),
            };
            drawing::update_point(transaction.0, entity.0, &request).map_err(host_error)
        }

        fn update_text(
            &mut self,
            transaction: TransactionId,
            entity: EntityId,
            request: &TextRequest,
        ) -> Result<(), HostError> {
            let request = WitTextRequest {
                position: wit_point(request.position),
                value: request.value.clone(),
                height: request.height,
                rotation_degrees: request.rotation_degrees,
                layer: request.layer.clone(),
            };
            drawing::update_text(transaction.0, entity.0, &request).map_err(host_error)
        }

        fn delete_entity(
            &mut self,
            transaction: TransactionId,
            entity: EntityId,
        ) -> Result<(), HostError> {
            drawing::delete_entity(transaction.0, entity.0).map_err(host_error)
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

    fn entity_view(view: WitEntityView) -> Result<EntityView, HostError> {
        let position = view
            .position
            .map(|point| {
                Point3::new(point.x, point.y, point.z)
                    .map_err(|error| HostError::new(error.to_string()))
            })
            .transpose()?;
        let kind = match view.kind {
            WitEntityKind::Line => EntityKind::Line,
            WitEntityKind::Arc => EntityKind::Arc,
            WitEntityKind::Circle => EntityKind::Circle,
            WitEntityKind::Point => EntityKind::Point,
            WitEntityKind::Text => EntityKind::Text,
            WitEntityKind::Other => EntityKind::Other,
        };
        Ok(EntityView {
            id: EntityId(view.id),
            kind,
            layer: view.layer,
            position,
            value: view.value,
        })
    }

    fn host_error(error: drawing::HostError) -> HostError {
        HostError::new(format!("{error:?}"))
    }

    export!(SurveyCraftExtension with_types_in self);
}
