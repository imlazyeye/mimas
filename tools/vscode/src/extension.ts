import { existsSync } from "node:fs";
import { delimiter, isAbsolute, join } from "node:path";
import { ExtensionContext, window, workspace } from "vscode";
import { LanguageClient, TransportKind } from "vscode-languageclient/node";

export function activate(context: ExtensionContext) {
    const command = workspace.getConfiguration("mimas").get<string>("serverPath", "mimas-lsp");
    if (!findExecutable(command)) {
        window.showWarningMessage(
            `mimas: language server \`${command}\` not found, so only syntax highlighting is on. ` +
                "Install mimas-lsp or point `mimas.serverPath` at it.",
        );
        return;
    }
    const client = new LanguageClient(
        "mimas",
        "mimas",
        { command, transport: TransportKind.stdio },
        { documentSelector: [{ scheme: "file", language: "mimas" }] },
    );
    context.subscriptions.push(client);
    client.start().catch((e) => window.showErrorMessage(`mimas: language server failed to start: ${e}`));
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
