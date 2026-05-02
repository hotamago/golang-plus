import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';

function getGoplusBinary(): string {
    const config = vscode.workspace.getConfiguration('goplus');
    return config.get<string>('binaryPath', 'goplus');
}

function getCwd(document: vscode.TextDocument): string {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (workspaceFolder) {
        return workspaceFolder.uri.fsPath;
    }
    return path.dirname(document.uri.fsPath);
}

export class GoplusFormattingProvider implements vscode.DocumentFormattingEditProvider {
    provideDocumentFormattingEdits(
        document: vscode.TextDocument,
        _options: vscode.FormattingOptions,
        _token: vscode.CancellationToken
    ): Promise<vscode.TextEdit[]> {
        return new Promise((resolve, reject) => {
            const binary = getGoplusBinary();
            const filePath = document.uri.fsPath;
            const cwd = getCwd(document);

            cp.execFile(
                binary,
                ['fmt', '--stdout', filePath],
                { cwd, timeout: 15000 },
                (err, stdout, stderr) => {
                    if (err) {
                        const errorMessage = stderr || err.message;
                        vscode.window.showWarningMessage(
                            `GoPlus format failed: ${errorMessage}`
                        );
                        resolve([]);
                        return;
                    }

                    if (!stdout || stdout.trim().length === 0) {
                        resolve([]);
                        return;
                    }

                    const fullRange = new vscode.Range(
                        document.lineAt(0).range.start,
                        document.lineAt(document.lineCount - 1).range.end
                    );

                    resolve([vscode.TextEdit.replace(fullRange, stdout)]);
                }
            );
        });
    }
}
