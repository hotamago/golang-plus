import * as vscode from 'vscode';
import * as path from 'path';

export function runInTerminal(document: vscode.TextDocument, commandArg: string): void {
    const config = vscode.workspace.getConfiguration('goplus');
    const binary = config.get<string>('binaryPath', 'goplus');
    
    let terminal = vscode.window.terminals.find(t => t.name === 'GoPlus');
    if (!terminal) {
        terminal = vscode.window.createTerminal('GoPlus');
    }
    terminal.show();
    
    const filePath = document.uri.fsPath;
    let cwd = path.dirname(filePath);
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(document.uri);
    if (workspaceFolder) {
        cwd = workspaceFolder.uri.fsPath;
    }
    
    // Convert to relatively clean paths or just use absolute
    const cmd = `${binary} ${commandArg} "${filePath}"`;
    
    terminal.sendText(`cd "${cwd}"`);
    terminal.sendText(cmd);
}

export async function buildFile(document: vscode.TextDocument): Promise<void> {
    runInTerminal(document, 'build');
}

export async function runFile(document: vscode.TextDocument): Promise<void> {
    runInTerminal(document, 'run');
}
