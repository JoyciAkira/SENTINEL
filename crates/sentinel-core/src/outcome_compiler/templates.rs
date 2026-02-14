//! Code templates per module type

use std::collections::HashMap;

/// Template manager for code generation
pub struct TemplateManager {
    templates: HashMap<String, String>,
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Web app module template
        templates.insert(
            "web_app_component".to_string(),
            r#"
import React from 'react';

export interface {{ModuleName}}Props {}

export const {{ModuleName}}: React.FC<{{ModuleName}}Props> = (props) => {
    return (
        <div>
            <h1>{{ModuleName}}</h1>
            {/* TODO: Implement */}
        </div>
    );
};
"#
            .to_string(),
        );

        // Backend service template
        templates.insert(
            "backend_service".to_string(),
            r#"
use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct {{ModuleName}}Response {
    pub message: String,
}

pub async fn {{ModuleName}}_handler() -> impl IntoResponse {
    Json({{ModuleName}}Response {
        message: "{{ModuleName}} - TODO: Implement".to_string(),
    })
}
"#
            .to_string(),
        );

        Self { templates }
    }

    pub fn get_template(&self, name: &str) -> Option<&String> {
        self.templates.get(name)
    }

    pub fn add_template(&mut self, name: String, template: String) {
        self.templates.insert(name, template);
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}
