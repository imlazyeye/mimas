use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use miette::{GraphicalReportHandler, GraphicalTheme, Report};
use parse::lex::{Lexer, TokKind};

/// A loaded `.mim` file.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct MimasScript {
    pub path: String,
    pub source: String,
}

impl MimasScript {
    /// Whether the file's first token is `module`.
    pub(crate) fn is_module(&self) -> bool {
        let mut lexer = Lexer::new(&self.source, 0, self.path.clone());
        matches!(lexer.next(), Some(tok) if matches!(tok.kind, TokKind::Module))
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

/// Runs a script's hooks for this entity. Inside a hook, `bevy::ecs::me()` is this entity.
#[derive(Component)]
pub struct Script(pub Handle<MimasScript>);

/// A script's compile error or runtime fault. `entity` is only set if it happened in a hook.
#[derive(Message, Debug, Clone)]
pub struct ScriptError {
    pub script: AssetId<MimasScript>,
    pub entity: Option<Entity>,
    pub message: String,
}

/// Logs and prints a script's error, and sends it as a [`ScriptError`].
pub(crate) fn report(
    world: &mut World,
    script: AssetId<MimasScript>,
    entity: Option<Entity>,
    err: &Report,
    message: String,
) {
    error!("{err}");
    // bevy's logger escapes the colors
    eprintln!("{err:?}");
    world.write_message(ScriptError {
        script,
        entity,
        message,
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
