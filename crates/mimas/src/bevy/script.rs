use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use miette::{GraphicalReportHandler, GraphicalTheme, Report};
use parse::lex::{Lexer, TokKind};

/// A `.mim` file. One that declares a module compiles into every script, and any other compiles
/// into a Vm of its own, shared by every entity whose [`Script`] points at it.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct MimasScript {
    pub path: String,
    pub source: String,
}

impl MimasScript {
    /// Whether the file's first token is `module`.
    pub(crate) fn is_module(&self) -> bool {
        let mut lexer = Lexer::new(&self.source, 0, self.path.clone());
        matches!(lexer.next(), Some(Ok(tok)) if matches!(tok.kind, TokKind::Module))
    }
}

/// Loads `.mim` files as [`MimasScript`]s.
#[derive(Default, TypePath)]
pub(crate) struct MimasScriptLoader;

impl AssetLoader for MimasScriptLoader {
    type Asset = MimasScript;
    type Settings = ();
    type Error = BevyError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<MimasScript, BevyError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(MimasScript {
            path: load_context.path().to_string(),
            source: String::from_utf8(bytes)?,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["mim"]
    }
}

/// Runs the script's hooks for this entity: `start` once, `update` every frame, `fixed_update`
/// every fixed tick, and `stop` when the component goes away. Inside them `script::entity()` is
/// this entity.
#[derive(Component)]
pub struct Script(pub Handle<MimasScript>);

/// A compile error or runtime fault, as plain text. It's also logged as one line and printed to
/// stderr as a full diagnostic. `entity` is set for faults in a hook and empty for compile errors
/// and faults in top-level code.
#[derive(Message, Debug, Clone)]
pub struct ScriptError {
    pub script: AssetId<MimasScript>,
    pub entity: Option<Entity>,
    pub message: String,
}

/// Logs a script's error as one line, prints the full diagnostic to stderr, and sends it as a
/// [`ScriptError`].
pub(crate) fn report(
    world: &mut World,
    script: AssetId<MimasScript>,
    entity: Option<Entity>,
    err: &Report,
) {
    error!("{err}");
    // the log formatter escapes ANSI codes, so the colored diagnostic goes around it
    eprintln!("{err:?}");
    world.write_message(ScriptError {
        script,
        entity,
        message: render(err),
    });
}

/// The diagnostic as plain, unwrapped text.
pub(crate) fn render(err: &Report) -> String {
    let mut text = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .with_wrap_lines(false)
        .render_report(&mut text, err.as_ref())
        .expect("writing to a String can't fail");
    text
}
