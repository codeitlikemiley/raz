use crate::{
    error::Result,
    parser::{RustParser, module_resolver::ModuleResolver},
    patterns::RunnableDetector,
    plugins::{ProjectContext, TargetRef},
    types::Runnable,
};

pub trait SourceAnalyzer: Send + Sync {
    fn id(&self) -> &'static str;

    fn analyze(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>>;
}

pub struct RustSourceAnalyzer;

impl SourceAnalyzer for RustSourceAnalyzer {
    fn id(&self) -> &'static str {
        "rust"
    }

    fn analyze(&self, ctx: &ProjectContext, line: Option<u32>) -> Result<Vec<TargetRef>> {
        if ctx.file_path.extension().and_then(|s| s.to_str()) != Some("rs") {
            return Ok(Vec::new());
        }

        let mut detector = RunnableDetector::new()?;
        let mut runnables = detector.detect_runnables(&ctx.file_path, line)?;

        if !runnables.is_empty() {
            resolve_module_paths(&mut runnables, &ctx.file_path)?;
        }

        Ok(runnables
            .into_iter()
            .map(|runnable| TargetRef::from_runnable(self.id(), runnable))
            .collect())
    }
}

fn resolve_module_paths(runnables: &mut [Runnable], file_path: &std::path::Path) -> Result<()> {
    let package_name = ModuleResolver::find_cargo_toml(file_path)
        .and_then(|cargo_toml| ModuleResolver::get_package_name_from_cargo_toml(&cargo_toml).ok());

    let resolver = if let Some(pkg) = package_name.as_deref() {
        ModuleResolver::with_package_name(pkg.to_string())
    } else {
        ModuleResolver::new()
    };

    let source = std::fs::read_to_string(file_path)?;
    let mut parser = RustParser::new()?;
    let scopes = parser.get_scopes(&source, file_path)?;

    for runnable in runnables {
        if let Ok(module_path) = resolver.resolve_module_path(file_path, &scopes, &runnable.scope) {
            runnable.module_path = module_path;
        }
    }

    Ok(())
}
