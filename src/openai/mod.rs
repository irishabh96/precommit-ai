use std::error::Error;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use surf::middleware::Middleware;
use surf::middleware::Next;
use surf::utils::async_trait;
use surf::Client;
use surf::Config;
use surf::Request;
use surf::Response;
use surf::StatusCode;
use surf::Url;
use thiserror::Error;

// const PROMPT: &str = "Write an insightful but concise Git commit message in a complete sentence in present tense for the following diff without prefacing it with anything:";
// const PROMPT: &str = "I have a git diff output that shows the changes made in a codebase. Your job is to check the code for any sytax error, missing fields in function, or any potential issue. ";
const PROMPT: &str = "**Prompt:** You are an AI code review assistant.Your task is to analyze a provided Git difference (diff) and provide a detailed but concise review of the changes.The user will input the diff, and you will generate feedback based on the following criteria: 1.**Code Quality**: Assess the readability, maintainability, and clarity of the code.Highlight any areas where the code can be improved in terms of style or structure.2.**Functionality**: Verify that the changes made do not introduce any bugs or regressions.Check if the new code adheres to the intended functionality described in the commit message.3.**Performance**: Evaluate any potential performance implications of the changes.Suggest optimizations if applicable.4.**Best Practices**: Identify whether the code follows industry best practices and design patterns.Provide suggestions for any deviations.5.**Testing**: Determine if there are adequate tests for the new code.Recommend additional tests if coverage is lacking.6.**Security**: Analyze the changes for any potential security vulnerabilities.Highlight areas that may require further scrutiny or improvement.7.**Documentation**: Check if the code is well-documented.Suggest any necessary updates to comments, README files, or external documentation.Please format your feedback in a clear, organized manner, separating each criterion into distinct sections.

";

struct BearerToken {
    token: String,
}

impl BearerToken {
    fn new(token: &str) -> Self {
        Self {
            token: String::from(token),
        }
    }
}

#[derive(Serialize)]
#[derive(Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: i32,
    top_p: f64,
    frequency_penalty: f64,
    presence_penalty: f64,
    n: i32,
    stream: bool,
}

#[derive(Serialize)]
#[derive(Debug)]
struct Message {
    role: String,
    content: String,
}

#[async_trait]
impl Middleware for BearerToken {
    async fn handle(
        &self,
        mut req: Request,
        client: Client,
        next: Next<'_>,
    ) -> surf::Result<Response> {
        req.insert_header("Authorization", format!("Bearer {}", self.token));
        let response: Response = next.run(req, client).await?;
        log::debug!("Response: {:?}", response);
        Ok(response)
    }
}

#[derive(Deserialize, Debug)]
pub(crate) struct ErrorWrapper {
    pub error: ErrorMessage,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct ErrorMessage {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("API returned an Error: {}", .0.message)]
    APIError(ErrorMessage),
    #[error("Base URL not set")]
    BaseUrlNotSet,
    #[error("The diff is too large for the OpenAI API. Try reducing the number of staged changes, or write your own commit message.")]
    DiffTooLarge,
}

impl From<ErrorMessage> for AppError {
    fn from(e: ErrorMessage) -> Self {
        AppError::APIError(e)
    }
}

impl From<String> for ErrorMessage {
    fn from(e: String) -> Self {
        ErrorMessage {
            message: e,
            error_type: String::from(""),
        }
    }
}

impl From<String> for AppError {
    fn from(e: String) -> Self {
        AppError::APIError(ErrorMessage::from(e))
    }
}

pub struct OpenAiClient {
    client: Client,
}

impl OpenAiClient {
    pub fn new(token: &str, base_url: &str) -> Result<Self, Box<dyn Error>> {
        let client: Client = Config::new()
            .set_base_url(Url::parse(base_url).unwrap())
            .try_into()?;
        Ok(Self {
            client: client.with(BearerToken::new(token)),
        })
    }

    async fn post<B, R>(&self, endpoint: &str, body: B) -> Result<R, Box<dyn Error>>
    where
        B: Serialize + std::fmt::Debug,
        R: DeserializeOwned,
    {
        let base_url = self
            .client
            .config()
            .base_url
            .as_ref()
            .ok_or(AppError::BaseUrlNotSet)?;

        let mut response = self
            .client
            .post(&format!("{}{}", base_url, endpoint))
            .body(surf::Body::from_json(&body)?)
            .await?;

            let response_body: serde_json::Value = response.body_json().await?;
        
            match response.status() {
                StatusCode::Ok => Ok(serde_json::from_value(response_body)?),
                _ => Err(Box::new(AppError::APIError(
                    response
                        .body_json::<ErrorWrapper>()
                        .await
                        .expect("The API has returned something funky")
                        .error,
                ))),
            }
    }

    async fn complete_chat(
        &self,
        messages: Vec<Message>,
    ) -> Result<CompletionResponse, Box<dyn Error>> {
        self.post("chat/completions", ChatRequest { model: String::from("gpt-3.5-turbo"), 
            messages,   
            temperature: 0.7,
            max_tokens: 500,
            top_p: 1.0,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stream: false,
            n: 1, 
        }).await
    }
    
    async fn create_chat_completion(&self, prompt: &str) -> Result<CompletionResponse, Box<dyn Error>> {
        let messages = vec![
            Message  {
                role: String::from("system"),
                content: PROMPT.to_string()
              },
            Message {
                role: String::from("user"),
                content: String::from(prompt),
            },
           
        ];
    
        let completion: CompletionResponse = self.complete_chat(messages).await?;
        Ok(completion)
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct CompletionRequest {
    model: String,
    prompt: String,
    temperature: f64,
    max_tokens: u64,
    top_p: f64,
    frequency_penalty: f64,
    presence_penalty: f64,
    stream: bool,
    n: u64,
}
#[derive(Deserialize, Debug, Clone)]
pub struct ResponseMessage {
    pub content: String
}
#[derive(Deserialize, Debug, Clone)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub text: Option<String>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Choice {
    pub message: ResponseMessage,
    pub text: Option<String>,
    pub index: u64,
    pub finish_reason: String,
}

fn sanitize_message(message: &str) -> String {
    message.to_string()
        // .trim()
        // .replace(['\n', '\r'], "")
        // .replace(r"\w\.$", "$1")
}

pub struct GitCommitMessageGenerator {
    openai_client: OpenAiClient,
}

impl GitCommitMessageGenerator {
    pub fn new(openai_client: OpenAiClient) -> Self {
        Self { openai_client }
    }

    pub async fn generate_commit_message(&self, diff: &str) -> Result<String, Box<dyn Error>> {
        let prompt: &str = &format!("**User Input**: ''' {} {}", diff, "'''");

        if prompt.len() > 40000 {
            return Err(AppError::DiffTooLarge.into());
        }

        println!("\n----- SYSTEM PROMPT -----\n{}\n----- USER PROMPT -----\n{}\n-------------------------\n", PROMPT, prompt);

        let completion: CompletionResponse = self.openai_client.create_chat_completion(prompt).await?;


        let message: String = sanitize_message(&completion.choices[0].message.content);
        Ok(message)
    }
}
