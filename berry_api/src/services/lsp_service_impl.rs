use crate::lsp_service::lsp_service_server::LspService;
use crate::lsp_service::*;
use tonic::{Request, Response, Status};

pub struct LspServiceImpl {}

impl LspServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[tonic::async_trait]
impl LspService for LspServiceImpl {
    async fn initialize(
        &self,
        request: Request<InitializeRequest>,
    ) -> Result<Response<InitializeResponse>, Status> {
        let req = request.into_inner();

        tracing::info!("🔧 LSP Initialize: language={}, root={}", req.language, req.root_uri);

        Ok(Response::new(InitializeResponse {
            success: true,
            error: None,
        }))
    }

    async fn get_completions(
        &self,
        request: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        let req = request.into_inner();

        tracing::info!(
            "💡 LSP Completions: file={}, line={}, char={}",
            req.file_path,
            req.position.as_ref().map(|p| p.line).unwrap_or(0),
            req.position.as_ref().map(|p| p.character).unwrap_or(0)
        );

        // Mock completions
        let items = vec![
            CompletionItem {
                label: "println!".to_string(),
                kind: Some(3), // Function
                detail: Some("macro".to_string()),
                documentation: Some("Print to console".to_string()),
                insert_text: Some("println!(\"{}\", )".to_string()),
                sort_text: None,
                filter_text: None,
            },
            CompletionItem {
                label: "String".to_string(),
                kind: Some(7), // Class
                detail: Some("struct".to_string()),
                documentation: Some("UTF-8 encoded string".to_string()),
                insert_text: Some("String".to_string()),
                sort_text: None,
                filter_text: None,
            },
        ];

        Ok(Response::new(CompletionResponse { items }))
    }

    async fn get_hover(
        &self,
        _request: Request<HoverRequest>,
    ) -> Result<Response<HoverResponse>, Status> {
        Ok(Response::new(HoverResponse {
            hover: Some(HoverInfo {
                contents: "Hover information".to_string(),
                range: None,
            }),
        }))
    }

    async fn goto_definition(
        &self,
        _request: Request<GotoDefinitionRequest>,
    ) -> Result<Response<LocationResponse>, Status> {
        Ok(Response::new(LocationResponse { locations: vec![] }))
    }

    async fn find_references(
        &self,
        _request: Request<FindReferencesRequest>,
    ) -> Result<Response<LocationsResponse>, Status> {
        Ok(Response::new(LocationsResponse { locations: vec![] }))
    }

    async fn get_diagnostics(
        &self,
        _request: Request<DiagnosticsRequest>,
    ) -> Result<Response<DiagnosticsResponse>, Status> {
        Ok(Response::new(DiagnosticsResponse {
            diagnostics: vec![],
        }))
    }

    async fn shutdown(
        &self,
        _request: Request<ShutdownRequest>,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn shutdown_all(
        &self,
        _request: Request<()>,
    ) -> Result<Response<()>, Status> {
        Ok(Response::new(()))
    }

    async fn get_theme(
        &self,
        _request: Request<ThemeRequest>,
    ) -> Result<Response<ThemeResponse>, Status> {
        // Return One Dark theme colors
        Ok(Response::new(ThemeResponse {
            theme_name: "One Dark".to_string(),
            colors: Some(ThemeColors {
                keyword: Some(RgbColor { r: 198, g: 120, b: 221 }),      // Purple
                function: Some(RgbColor { r: 97, g: 175, b: 239 }),      // Blue
                type_color: Some(RgbColor { r: 229, g: 192, b: 123 }),   // Yellow
                string: Some(RgbColor { r: 152, g: 195, b: 121 }),       // Green
                number: Some(RgbColor { r: 209, g: 154, b: 102 }),       // Orange
                comment: Some(RgbColor { r: 92, g: 99, b: 112 }),        // Gray
                macro_color: Some(RgbColor { r: 86, g: 182, b: 194 }),   // Cyan
                attribute: Some(RgbColor { r: 209, g: 154, b: 102 }),    // Orange
                constant: Some(RgbColor { r: 209, g: 154, b: 102 }),     // Orange
                lifetime: Some(RgbColor { r: 229, g: 192, b: 123 }),     // Yellow
                background: Some(RgbColor { r: 40, g: 44, b: 52 }),      // Dark
                foreground: Some(RgbColor { r: 171, g: 178, b: 191 }),   // Light Gray
                selection: Some(RgbColor { r: 61, g: 66, b: 77 }),       // Darker Gray
                cursor: Some(RgbColor { r: 97, g: 175, b: 239 }),        // Blue
                line_number: Some(RgbColor { r: 92, g: 99, b: 112 }),    // Gray
            }),
        }))
    }
}
