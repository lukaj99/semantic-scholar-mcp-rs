//! Author network discovery tool.

use std::collections::HashMap;

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::models::{AuthorNetworkInput, ResponseFormat};

/// Author collaboration network tool.
pub struct AuthorNetworkTool;

#[async_trait::async_trait]
impl McpTool for AuthorNetworkTool {
    fn name(&self) -> &'static str {
        "author_network"
    }

    fn description(&self) -> &'static str {
        "Discover author collaboration networks. Analyzes papers to find \
         frequent collaborators and build a collaboration graph."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "authorId": {
                    "type": "string",
                    "description": "Semantic Scholar author ID"
                },
                "depth": {
                    "type": "integer",
                    "default": 1,
                    "minimum": 1,
                    "maximum": 2,
                    "description": "Collaboration depth (1 = direct, 2 = collaborators of collaborators)"
                },
                "minSharedPapers": {
                    "type": "integer",
                    "default": 2,
                    "description": "Minimum shared papers to include a collaborator"
                },
                "maxCollaborators": {
                    "type": "integer",
                    "default": 50,
                    "description": "Maximum collaborators to return"
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["authorId"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: AuthorNetworkInput = serde_json::from_value(input)?;

        // Get author info
        let author_info =
            ctx.client.get_author(&params.author_id).await.map_err(ToolError::from)?;
        let author_name = author_info.name_or_default().to_string();

        // Get author's papers
        let mut papers = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            let result = ctx
                .client
                .search_papers(
                    &format!("author:{}", params.author_id),
                    offset,
                    limit,
                    fields::DEFAULT,
                )
                .await;

            match result {
                Ok(search_result) => {
                    papers.extend(search_result.data);
                    if search_result.next.is_none() || papers.len() >= 200 {
                        break;
                    }
                    offset = search_result.next.unwrap_or(offset + limit);
                }
                Err(_) => break,
            }
        }

        // Count co-author occurrences
        #[derive(Default)]
        struct CoauthorData {
            name: String,
            shared_papers: i32,
            paper_ids: Vec<String>,
        }

        let mut coauthor_counts: HashMap<String, CoauthorData> = HashMap::new();

        for paper in &papers {
            for author in &paper.authors {
                if let Some(ref author_id) = author.author_id {
                    if author_id == &params.author_id {
                        continue;
                    }

                    let entry = coauthor_counts.entry(author_id.clone()).or_default();
                    entry.name = author.name_or_default().to_string();
                    entry.shared_papers += 1;
                    if entry.paper_ids.len() < 5 {
                        entry.paper_ids.push(paper.paper_id.clone());
                    }
                }
            }
        }

        // Filter by min_shared_papers and build collaborator list
        let mut collaborators: Vec<_> = coauthor_counts
            .into_iter()
            .filter(|(_, data)| data.shared_papers >= params.min_shared_papers)
            .map(|(id, data)| {
                json!({
                    "id": id,
                    "name": data.name,
                    "shared_papers": data.shared_papers,
                    "paperIds": data.paper_ids
                })
            })
            .collect();

        // Sort by shared papers descending
        collaborators.sort_by(|a, b| {
            let a_shared = a["shared_papers"].as_i64().unwrap_or(0);
            let b_shared = b["shared_papers"].as_i64().unwrap_or(0);
            b_shared.cmp(&a_shared)
        });

        // Limit to max_collaborators
        collaborators.truncate(params.max_collaborators as usize);

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Author Collaboration Network\n\n\
                     **Author:** {} ({})\n\
                     **Total papers analyzed:** {}\n\
                     **Collaborators found:** {}\n\
                     **Min shared papers filter:** {}\n\n\
                     ---\n\n\
                     ## Top Collaborators\n\n",
                    author_name,
                    params.author_id,
                    papers.len(),
                    collaborators.len(),
                    params.min_shared_papers
                );

                for (i, collab) in collaborators.iter().enumerate() {
                    output.push_str(&format!(
                        "**{}. {}** - {} shared papers\n   - ID: {}\n\n",
                        i + 1,
                        collab["name"].as_str().unwrap_or("Unknown"),
                        collab["shared_papers"],
                        collab["id"].as_str().unwrap_or("")
                    ));
                }

                if collaborators.is_empty() {
                    output.push_str("*No collaborators found matching the criteria.*");
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "author": {
                    "id": params.author_id,
                    "name": author_name
                },
                "total_papers": papers.len(),
                "total_collaborators": collaborators.len(),
                "minSharedPapers": params.min_shared_papers,
                "collaborators": collaborators
            }))?),
        }
    }
}
