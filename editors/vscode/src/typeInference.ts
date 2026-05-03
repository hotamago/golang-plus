import * as vscode from 'vscode';

export async function inferTypeOfVariable(document: vscode.TextDocument, position: vscode.Position, variableName: string): Promise<string | undefined> {
    let rhs: string | undefined;

    // Search upwards from the given position to find the variable definition
    for (let i = position.line; i >= 0; i--) {
        const text = document.lineAt(i).text;
        
        // let x = RHS or var x = RHS
        const letMatch = text.match(new RegExp(`^\\s*(?:let|var|const)\\s+${variableName}\\s*=\\s*(.+)$`));
        if (letMatch) {
            rhs = letMatch[1];
            break;
        }

        // x := RHS
        const walrusMatch = text.match(new RegExp(`^\\s*${variableName}\\s*:=\\s*(.+)$`));
        if (walrusMatch) {
            rhs = walrusMatch[1];
            break;
        }
        
        // for idx, value := range RHS
        const forRangeMatch = text.match(new RegExp(`^\\s*for\\s+([a-zA-Z_]\\w*)\\s*,\\s*([a-zA-Z_]\\w*)\\s*:=\\s*range\\s+(.+)$`));
        if (forRangeMatch) {
            const idxName = forRangeMatch[1];
            const valName = forRangeMatch[2];
            const rangeTarget = forRangeMatch[3];
            
            if (variableName === idxName) {
                return 'int'; // Usually int, unless it's a map. Simplified here.
            }
            if (variableName === valName) {
                // If ranging over an array of type []T, value is T
                const targetType = await inferTypeFromExpression(document, rangeTarget.trim());
                if (targetType?.startsWith('[]')) {
                    return targetType.substring(2);
                }
                return 'unknown element type';
            }
        }

        // Check for function parameter
        const fnPattern = /^\s*fn(?: \w+)?\s+\w+\((.*)\)/;
        const match = text.match(fnPattern);
        if (match) {
            const params = match[1];
            const paramPattern = new RegExp(`\\b${variableName}\\s*:`);
            if (paramPattern.test(params)) {
                const typeMatch = params.match(new RegExp(`\\b${variableName}\\s*:\\s*([^,)]+)`));
                if (typeMatch) {
                    return typeMatch[1].trim();
                }
            }
            break; // Stop at function boundary
        }
    }

    if (rhs) {
        return await inferTypeFromExpression(document, rhs.trim());
    }

    return undefined;
}

export async function inferTypeFromExpression(document: vscode.TextDocument, expr: string): Promise<string | undefined> {
    // String literal
    if (/^".*"$/.test(expr) || /^`.*`$/.test(expr)) return 'string';
    
    // Float literal
    if (/^\d+\.\d+$/.test(expr)) return 'float64';
    
    // Int literal
    if (/^\d+$/.test(expr)) return 'int';
    
    // Bool literal
    if (expr === 'true' || expr === 'false') return 'bool';
    
    // Map literal
    const mapMatch = expr.match(/^(map\[.+\][a-zA-Z0-9_.*]+)/);
    if (mapMatch) return mapMatch[1];
    
    // Slice literal
    const sliceMatch = expr.match(/^(\[\].+?)\s*\{/);
    if (sliceMatch) return sliceMatch[1];
    
    // Struct initialization Account{...}
    const structMatch = expr.match(/^([A-Z][a-zA-Z0-9_]*)\s*\{/);
    if (structMatch) return structMatch[1];
    
    // Type conversion or Interface wrapping e.g. Labeler(account)
    const convMatch = expr.match(/^([A-Z][a-zA-Z0-9_]*)\s*\([^)]+\)$/);
    if (convMatch) return convMatch[1];

    // Function call e.g. buildBuckets(values)
    const fnCallMatch = expr.match(/^([a-zA-Z_]\w*)\s*\(/);
    if (fnCallMatch) {
        const fnName = fnCallMatch[1];
        if (fnName === 'make') {
            const makeMatch = expr.match(/make\s*\(\s*([^,]+)/);
            if (makeMatch) return makeMatch[1].trim();
        }
        
        // Search workspace for the function definition to find its return type
        return await getFunctionReturnType(fnName);
    }

    return undefined;
}

async function getFunctionReturnType(fnName: string): Promise<string | undefined> {
    const uris = await vscode.workspace.findFiles('{**/*.gp,**/*.go}', '**/node_modules/**');
    const docs = new Set(vscode.workspace.textDocuments.filter(d => d.languageId === 'goplus' || d.languageId === 'go'));
    for (const uri of uris) {
        const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
        if (!openDoc) {
            try {
                const doc = await vscode.workspace.openTextDocument(uri);
                docs.add(doc);
            } catch {
                // Ignore
            }
        }
    }

    // Pattern: fn funcName(...) -> ReturnType
    // or fn funcName(...) ReturnType (Go-style)
    // or func funcName(...) ReturnType
    const pattern = new RegExp(`^(?:fn|func)\\s+${fnName}\\s*\\([^)]*\\)(?:\\s*->)?\\s*([^\\{\\!\\n]+)`);

    for (const doc of docs) {
        for (let i = 0; i < doc.lineCount; i++) {
            const text = doc.lineAt(i).text.trim();
            if (text.startsWith('fn ') || text.startsWith('func ')) {
                const match = text.match(pattern);
                if (match) {
                    return match[1].trim();
                }
                
                // If it's a void function `fn funcName() {`
                if (text.match(new RegExp(`^(?:fn|func)\\s+${fnName}\\s*\\([^)]*\\)\\s*\\{`))) {
                    return 'void';
                }
            }
        }
    }

    return undefined;
}
