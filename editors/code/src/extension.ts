import path = require("path");
import * as vscode from "vscode";
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Middleware,
} from "vscode-languageclient/node";

const middleware: Middleware = {
  provideCompletionItem: async (document, position, context, token, next) => {
    await new Promise((r) => setTimeout(r, 1));

    const result = await next(document, position, context, token);

    // Get the text the user is typing at the cursor position
    const line = document.lineAt(position.line).text;
    const typedPrefix = line.slice(0, position.character).split(/\W+/).pop() || "";

    // Create case-insensitive regex from prefix
    const regex = new RegExp("^" + typedPrefix, "i");

    const matchesRegex = (item: any) =>
      regex.test(item.label || "");

    // In case the server returns the array directly
    if (Array.isArray(result)) {
      const filtered = result.filter(matchesRegex).slice(0, 100);
      return filtered;
    }

    // In case the server returns `CompletionList`
    if (result && Array.isArray(result.items)) {
      result.items = result.items.filter(matchesRegex).slice(0, 100);
      return result;
    }

    return result;
  },
};
let client: LanguageClient;

function serverExecutable() {
  const platform = process.platform;
  const arch = process.arch;
  const exe = platform === 'win32' ? 'asm-lsp.exe' : 'asm-lsp';
  return path.join(__dirname, '..', 'bin', `${platform}-${arch}`, exe);
}

export function activate(_: vscode.ExtensionContext) {
  const serverPath = process.env.SERVER_PATH || serverExecutable();

  if (!serverPath) {
    vscode.window.showErrorMessage(
      "[asm-lsp] Environment variable SERVER_PATH is not set. Please set it to the asm-lsp binary."
    );
    return;
  }

  const run: Executable = {
    command: serverPath,
    options: {
      env: {
        ...process.env,
      },
    },
  };

  const serverOptions: ServerOptions = {
    run,
    debug: run, // Use the same configuration for both run and debug modes
    // NOTE: Debug disabled by not declaring `debug`
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "asm" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/.clientrc"),
    },
    middleware,
  };

  try {
    client = new LanguageClient(
      "asm-lsp",
      "ASM Language Server",
      serverOptions,
      clientOptions
    );

    client
      .start()
      .then(() => {
        console.log("asm-lsp started at:", serverPath);
      })
      .catch((err) => {
        vscode.window.showErrorMessage(
          `[asm-lsp] Failed to start language server: ${err.message}`
        );
        console.error(err);
      });
  } catch (err: any) {
    vscode.window.showErrorMessage(
      `[asm-lsp] Unexpected error while starting language server: ${err.message}`
    );
  }
}

export function deactivate(): Thenable<void> | undefined {
  return client ? client.stop() : undefined;
}
