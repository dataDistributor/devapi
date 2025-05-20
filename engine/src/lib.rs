use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkflowStep {
    pub method: String,
    pub url: String,
    pub body: Option<Value>,
    pub headers: Option<Vec<(String, String)>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Workflow {
    pub steps: Vec<WorkflowStep>,
}

pub async fn execute_workflow(workflow: Workflow) -> Result<Vec<Value>, String> {
    let client = Client::new();
    let mut results = Vec::new();

    for step in workflow.steps {
        let method = step.method.parse::<Method>().map_err(|e| e.to_string())?;
        let mut request = client.request(method, &step.url);

        if let Some(headers) = &step.headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }

        if let Some(body) = &step.body {
            request = request.json(body);
        }

        let response = request.send().await.map_err(|e| e.to_string())?;
        let json = response.json::<Value>().await.map_err(|e| e.to_string())?;
        results.push(json);
    }

    Ok(results)
}
