use crate::config::OutputFormat;
use anyhow::Result;
use comfy_table::{Table, presets::UTF8_FULL};
use serde_json::Value;

pub struct OutputManager {
    format: OutputFormat,
}

impl OutputManager {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn print_value(&self, value: &Value) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(value)?);
            }
            OutputFormat::Yaml => {
                // Simplified YAML output, using JSON as replacement for now
                // In actual project, serde_yaml dependency can be added
                println!("{}", serde_json::to_string_pretty(value)?);
            }
            OutputFormat::Table => {
                self.print_as_table(value)?;
            }
        }
        Ok(())
    }

    pub fn print_master_keys(&self, keys: &[sealbox_server::repo::MasterKey]) -> Result<()> {
        match self.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(keys)?);
            }
            OutputFormat::Yaml => {
                for key in keys {
                    println!("- id: {}", key.id);
                    println!("  status: {:?}", key.status);
                    println!("  created_at: {}", key.created_at);
                    println!(
                        "  public_key: {}",
                        if key.public_key == "[HIDDEN]" {
                            "[HIDDEN]"
                        } else {
                            &key.public_key
                        }
                    );
                    println!();
                }
            }
            OutputFormat::Table => {
                let mut table = Table::new();
                table.load_style(UTF8_FULL);
                table.set_header(vec!["ID", "Status", "Created At", "Public Key"]);

                for key in keys {
                    let created_at = time::OffsetDateTime::from_unix_timestamp(key.created_at)
                        .map(|dt| {
                            dt.format(&time::format_description::well_known::Rfc2822)
                                .unwrap_or_else(|_| dt.to_string())
                        })
                        .unwrap_or_else(|_| key.created_at.to_string());

                    table.add_row(vec![
                        key.id.to_string(),
                        format!("{:?}", key.status),
                        created_at,
                        if key.public_key == "[HIDDEN]" {
                            "[HIDDEN]".to_string()
                        } else {
                            format!("{}...", &key.public_key[..20.min(key.public_key.len())])
                        },
                    ]);
                }

                println!("{table}");
            }
        }
        Ok(())
    }

    fn print_as_table(&self, value: &Value) -> Result<()> {
        let mut table = Table::new();
        table.load_style(UTF8_FULL);

        match value {
            Value::Object(obj) => {
                table.set_header(vec!["Key", "Value"]);
                for (k, v) in obj {
                    table.add_row(vec![k.clone(), self.value_to_string(v)]);
                }
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    println!("Empty array");
                    return Ok(());
                }

                // Try to display as object array
                if let Some(Value::Object(first_obj)) = arr.first() {
                    let headers: Vec<String> = first_obj.keys().cloned().collect();
                    table.set_header(headers.clone());

                    for item in arr {
                        if let Value::Object(obj) = item {
                            let row: Vec<String> = headers
                                .iter()
                                .map(|h| {
                                    obj.get(h)
                                        .map_or("".to_string(), |v| self.value_to_string(v))
                                })
                                .collect();
                            table.add_row(row);
                        }
                    }
                } else {
                    // Simple array
                    table.set_header(vec!["Index", "Value"]);
                    for (i, v) in arr.iter().enumerate() {
                        table.add_row(vec![i.to_string(), self.value_to_string(v)]);
                    }
                }
            }
            _ => {
                table.set_header(vec!["Value"]);
                table.add_row(vec![self.value_to_string(value)]);
            }
        }

        println!("{table}");
        Ok(())
    }

    fn value_to_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            _ => serde_json::to_string(value).unwrap_or_else(|_| "Unable to display".to_string()),
        }
    }

    pub fn print_success(&self, message: &str) {
        println!("✅ {message}");
    }

    pub fn print_warning(&self, message: &str) {
        println!("⚠️  {message}");
    }

    pub fn print_info(&self, message: &str) {
        println!("ℹ️  {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_print_value_json() {
        let output = OutputManager::new(OutputFormat::Json);
        let value = json!({"test": "value", "number": 42});

        // This test mainly verifies no panic occurs, actual output needs manual verification
        assert!(output.print_value(&value).is_ok());
    }

    #[test]
    fn test_value_to_string() {
        let output = OutputManager::new(OutputFormat::Json);

        assert_eq!(output.value_to_string(&json!("test")), "test");
        assert_eq!(output.value_to_string(&json!(42)), "42");
        assert_eq!(output.value_to_string(&json!(true)), "true");
        assert_eq!(output.value_to_string(&json!(null)), "null");
    }
}
