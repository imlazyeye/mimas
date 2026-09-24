import { existsSync } from "node:fs";
import { delimiter, isAbsolute, join, relative } from "node:path";
import { commands, ConfigurationTarget, ExtensionContext, window, workspace } from "vscode";
import { LanguageClient, TransportKind } from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let extensionPath = "";

export function activate(context: ExtensionContext) {
    extensionPath = context.extensionPath;
    context.subscriptions.push(
        commands.registerCommand("mimas.restartServer", restartServer),
        commands.registerCommand("mimas.selectApiManifest", selectApiManifest),
        workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration("mimas.apiPath") || e.affectsConfiguration("mimas.serverPath")) {
                restartServer();
            }
        }),
    );
    startServer();
}

export function deactivate() {
    return client?.stop();
}

function startServer() {
    const config = workspace.getConfiguration("mimas");
    const command = config.get<string>("serverPath") || defaultServer();
    if (!findExecutable(command)) {
        window.showWarningMessage(
            `mimas: language server \`${command}\` not found, so only syntax highlighting is on. ` +
                "Install mimas-lsp or point `mimas.serverPath` at it.",
        );
        return;
    }
    client = new LanguageClient(
        "mimas",
        "mimas",
        { command, transport: TransportKind.stdio },
        {
            documentSelector: [{ scheme: "file", language: "mimas" }],
            initializationOptions: {
                apiPath: config.get<string>("apiPath") || undefined,
            },
        },
    );
    client.start().catch((e) => window.showErrorMessage(`mimas: language server failed to start: ${e}`));
}

async function restartServer() {
    const old = client;
    client = undefined;
    // a server that never started refuses to stop
    await old?.dispose().catch(() => {});
    startServer();
}

// saved relative to the workspace folder when it's inside one, so the setting survives a moved checkout
async function selectApiManifest() {
    const root = workspace.workspaceFolders?.[0]?.uri;
    const picked = await window.showOpenDialog({
        defaultUri: root,
        canSelectMany: false,
        filters: { "API manifest": ["json"] },
        openLabel: "Use Manifest",
    });
    if (!picked?.[0]) {
        return;
    }
    const path = picked[0].fsPath;
    const inRoot = root && relative(root.fsPath, path);
    const value = inRoot && !inRoot.startsWith("..") && !isAbsolute(inRoot) ? inRoot : path;
    const target = root ? ConfigurationTarget.Workspace : ConfigurationTarget.Global;
    await workspace.getConfiguration("mimas").update("apiPath", value, target);
}

// the platform-specific builds of the extension ship the server, the universal one doesn't
function defaultServer(): string {
    const bundled = join(extensionPath, "server", process.platform === "win32" ? "mimas-lsp.exe" : "mimas-lsp");
    return existsSync(bundled) ? bundled : "mimas-lsp";
}

// a path is checked where the server will be spawned from (the workspace folder), a bare name on PATH
function findExecutable(command: string): boolean {
    if (command.includes("/") || command.includes("\\")) {
        const cwd = workspace.workspaceFolders?.[0]?.uri.fsPath;
        return existsSync(isAbsolute(command) || !cwd ? command : join(cwd, command));
    }
    const exts = process.platform === "win32" ? ["", ".exe", ".cmd", ".bat"] : [""];
    return (process.env.PATH ?? "")
        .split(delimiter)
        .some((dir) => exts.some((ext) => existsSync(join(dir, command + ext))));
}
