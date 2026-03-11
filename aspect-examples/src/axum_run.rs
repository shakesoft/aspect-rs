use aspect_core::{
    Aspect, AspectError, AsyncAspect, AsyncJoinPoint, JoinPoint, ProceedingJoinPoint,
};
use aspect_macros::{aspect, async_aspect};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, extract::Query, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::net::SocketAddr;
use thiserror::Error;
use tokio::time::Instant;

#[derive(Debug, Deserialize, Clone)]
struct HelloRequest {
    pub page_no: u64,
    pub page_size: u64,
    pub mobile: Option<String>,    //手机
    pub user_name: Option<String>, //姓名
    #[serde(default = "default_status")]
    pub status: Option<i8>, //状态(1:正常，0:禁用)
    pub dept_id: Option<i64>,      //部门ID
}

fn default_status() -> Option<i8> {
    Some(2)
}
#[derive(Debug, Serialize)]
struct HelloResponse {
    message: String,
    name: String,
    age: u64,
}

fn build_hello_response(name: String, age: u64) -> HelloResponse {
    HelloResponse {
        message: format!("Hello, {}!", name),
        name,
        age,
    }
}

#[aspect(Logger1)]
async fn hello(Query(params): Query<HelloRequest>) -> impl IntoResponse {
    let res = add(1, 2).await;
    println!("add result: {res}");
    let result = build_hello_response(
        params.user_name.clone().unwrap_or("Guest".to_string()),
        params.page_no,
    );
    ok_result_data(result).into_response()
}

#[async_aspect(Logger)]
async fn add(num1: i32, num2: i32) -> i32 {
    sub(num1, num2).await
}

#[aspect(Logger2)]
async fn sub(num1: i32, num2: i32) -> i32 {
    num1 + num2
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/hello", get(hello));

    let addr: SocketAddr = "127.0.0.1:3000".parse().expect("invalid bind address");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind tcp listener");

    println!("axum_site listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .expect("server runtime error");
}

#[derive(Default)]
pub struct Logger;

impl AsyncAspect for Logger {
    async fn before(&self, ctx: &AsyncJoinPoint) {
        let num1 = ctx
            .args
            .get(0)
            .and_then(|arg| arg.downcast_ref::<i32>())
            .copied();
        let num2 = ctx
            .args
            .get(1)
            .and_then(|arg| arg.downcast_ref::<i32>())
            .copied();

        println!("async before add args: num1={num1:?}, num2={num2:?}");
    }

    async fn after(&self, ctx: &AsyncJoinPoint, result: &(dyn Any + Send + Sync)) {
        let num1 = ctx
            .args
            .get(0)
            .and_then(|arg| arg.downcast_ref::<i32>())
            .copied();
        let num2 = ctx
            .args
            .get(1)
            .and_then(|arg| arg.downcast_ref::<i32>())
            .copied();
        let value = result.downcast_ref::<i32>().copied();

        println!("async after add args: num1={num1:?}, num2={num2:?}, result={value:?}");
    }
}

#[derive(Default)]
pub struct Logger1;

impl Aspect for Logger1 {
    fn before(&self, ctx: &JoinPoint) {
        let arg1 = ctx.args.get(0).unwrap().downcast_ref::<HelloRequest>();
        println!("before {}", arg1.unwrap().mobile.as_ref().unwrap());
    }

    fn after(&self, _ctx: &JoinPoint, _result: &dyn Any) {
        let arg1 = _ctx.args.get(0).unwrap().downcast_ref::<HelloRequest>();
        println!("after{}", arg1.unwrap().mobile.as_ref().unwrap());
    }
}

#[derive(Default)]
pub struct Logger2;

impl Aspect for Logger2 {
    fn around(&self, pjp: ProceedingJoinPoint) -> Result<Box<dyn Any>, AspectError> {
        let start = Instant::now();
        let function_name = pjp.context().function_name;
        let result = pjp.proceed();
        let elapsed = start.elapsed();
        println!("{} took {:?}", function_name, elapsed);
        match &result {
            Ok(val) => println!("{} executed successfully", function_name),
            Err(e) => println!("{} execution failed: {:?}", function_name, e),
        };
        result
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    // #[error("Failed to complete an HTTP request")]
    // Http { #[from] source: reqwest::Error },
    //
    #[error("Failed to read the cache file")]
    DiskCacheRead { source: std::io::Error },
    //
    // #[error("Failed to update the cache file")]
    // DiskCacheWrite { source: std::io::Error },
    #[error("jwt：{0}")]
    JwtTokenError(String),

    #[error("业务异常: {0}")]
    BusinessError(&'static str),

    #[error("验证异常: {0}")]
    ValidationError(String),

    #[error("内部异常: {0}")]
    InternalError(&'static str),
}

pub type AppResult<T> = Result<T, AppError>;

#[async_trait]
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let response = BaseResponse {
            msg: self.to_string(),
            code: 1,
            data: Some("None".to_string()),
        };

        match self {
            AppError::DiskCacheRead { source: _ } => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
            AppError::JwtTokenError(_msg) => {
                (StatusCode::UNAUTHORIZED, Json(response)).into_response()
            }
            AppError::BusinessError(_msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
            AppError::ValidationError(_msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
            AppError::InternalError(_msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
            }
        }
    }
}

impl AppError {
    pub fn default() -> AppError {
        AppError::InternalError("服务器发生内部异常，请稍后再试")
    }
    pub fn interrupt() -> AppResult<Json<BaseResponse<()>>> {
        Err(AppError::default())
    }

    pub fn build_validation_error_message(e: &validator::ValidationErrors) -> String {
        e.field_errors()
            .iter()
            .map(|(field, errors)| {
                let messages: Vec<String> = errors
                    .iter()
                    .map(|error| {
                        if let Some(message) = &error.message {
                            message.to_string()
                        } else {
                            format!("字段 '{}' 验证失败", field)
                        }
                    })
                    .collect();
                messages.join(", ")
            })
            .collect::<Vec<String>>()
            .join("; ")
    }

    pub fn validation_error(e: &validator::ValidationErrors) -> AppError {
        AppError::ValidationError(Self::build_validation_error_message(e))
    }
}

// 统一返回vo
#[derive(Serialize, Debug, Clone)]
pub struct BaseResponse<T> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
}

#[derive(Serialize, Debug, Clone)]
pub struct EmptyResponse {
    pub code: i32,
    pub msg: String,
    pub data: Option<()>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ResponsePage<T> {
    pub code: i32,
    pub msg: String,
    pub total: u64,
    pub success: bool,
    pub data: Option<T>,
}

pub fn ok() -> AppResult<Json<EmptyResponse>> {
    Ok(Json(EmptyResponse {
        msg: "操作成功".to_string(),
        code: 0,
        data: Some(()),
    }))
}

pub fn ok_result() -> AppResult<Json<BaseResponse<String>>> {
    ok_result_msg("操作成功")
}

pub fn ok_result_msg(msg: &str) -> AppResult<Json<BaseResponse<String>>> {
    Ok(Json(BaseResponse {
        msg: msg.to_string(),
        code: 0,
        data: Some("None".to_string()),
    }))
}

pub fn ok_result_data<T>(data: T) -> AppResult<Json<BaseResponse<T>>> {
    Ok(Json(BaseResponse {
        msg: "操作成功".to_string(),
        code: 0,
        data: Some(data),
    }))
}

pub fn ok_result_page<T>(data: T, total: u64) -> AppResult<Json<ResponsePage<T>>> {
    Ok(Json(ResponsePage {
        msg: "操作成功".to_string(),
        code: 0,
        success: true,
        data: Some(data),
        total,
    }))
}

pub fn err_result_msg(msg: &str) -> AppResult<Json<BaseResponse<String>>> {
    Ok(Json(BaseResponse {
        msg: msg.to_string(),
        code: 1,
        data: Some("None".to_string()),
    }))
}
