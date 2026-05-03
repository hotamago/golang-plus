import * as vscode from 'vscode';
import { inferTypeOfVariable } from './typeInference';

export class GoplusCompletionProvider implements vscode.CompletionItemProvider {
    async provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
        context: vscode.CompletionContext
    ): Promise<vscode.CompletionItem[] | vscode.CompletionList> {
        const linePrefix = document.lineAt(position).text.substring(0, position.character);
        
        // If typing EnumName::, suggest enum variants
        const namespaceMatch = linePrefix.match(/([a-zA-Z_][a-zA-Z0-9_]*)::$/);
        if (namespaceMatch) {
            const enumName = namespaceMatch[1];
            return await this.getEnumVariants(enumName);
        }

        // If typing Object., suggest methods
        const methodMatch = linePrefix.match(/([a-zA-Z_][a-zA-Z0-9_]*)\.$/);
        if (methodMatch) {
            const objName = methodMatch[1];
            const objType = await inferTypeOfVariable(document, position, objName);
            if (objType) {
                return await this.getMethodsForType(objType);
            }
        }

        // If we are inside @derive(...), suggest derive kinds
        if (linePrefix.includes('@derive(')) {
            const closingParen = linePrefix.indexOf(')');
            if (closingParen === -1 || closingParen >= position.character) {
                return [
                    this.createDeriveCompletion('String', 'Generates String() string for fmt.Println'),
                    this.createDeriveCompletion('Debug', 'Generates Debug() string'),
                    this.createDeriveCompletion('Equal', 'Generates Equal(other) bool'),
                    this.createDeriveCompletion('JsonMarshal', 'Generates MarshalJSON() ([]byte, error)'),
                    this.createDeriveCompletion('JsonUnmarshal', 'Generates UnmarshalJSON([]byte) error')
                ];
            }
        }

        // If typing a decorator, suggest decorators
        if (linePrefix.endsWith('@')) {
            return [
                this.createDecoratorCompletion('log', 'Auto-logs function entry/exit'),
                this.createDecoratorCompletion('retry(times, backoff)', 'Retries function on error'),
                this.createDecoratorCompletion('memoize', 'Caches function results'),
                this.createDecoratorCompletion('derive(Kinds...)', 'Generates methods')
            ];
        }

        const items: vscode.CompletionItem[] = [];

        // Keywords
        const keywords = ['fn', 'struct', 'enum', 'impl', 'match', 'return', 'let', 'mut', 'const', 'import', 'package', 'self', 'if', 'for', 'defer'];
        for (const kw of keywords) {
            const item = new vscode.CompletionItem(kw, vscode.CompletionItemKind.Keyword);
            items.push(item);
        }

        // Built-in types
        const types = ['string', 'int', 'int8', 'int16', 'int32', 'int64', 'uint', 'uint8', 'uint16', 'uint32', 'uint64', 'byte', 'rune', 'float32', 'float64', 'bool', 'any', 'error'];
        for (const ty of types) {
            const item = new vscode.CompletionItem(ty, vscode.CompletionItemKind.TypeParameter);
            items.push(item);
        }
        
        // Scan current document for definitions to recommend (functions, structs, enums)
        const docText = document.getText();
        const definitionRegex = /^\s*(?:fn(?:\s+mut)?|struct|enum|const|let|var)\s+([a-zA-Z_][a-zA-Z0-9_]*)/gm;
        let match;
        const seen = new Set<string>();
        while ((match = definitionRegex.exec(docText)) !== null) {
            const name = match[1];
            if (!seen.has(name)) {
                seen.add(name);
                items.push(new vscode.CompletionItem(name, vscode.CompletionItemKind.Reference));
            }
        }

        return items;
    }

    private async getEnumVariants(enumName: string): Promise<vscode.CompletionItem[]> {
        const uris = await vscode.workspace.findFiles('**/*.gp', '**/node_modules/**');
        const docs = new Set(vscode.workspace.textDocuments.filter(d => d.languageId === 'goplus'));
        for (const uri of uris) {
            const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
            if (!openDoc) {
                try {
                    const doc = await vscode.workspace.openTextDocument(uri);
                    docs.add(doc);
                } catch {
                    // Ignore errors opening files
                }
            }
        }

        const items: vscode.CompletionItem[] = [];
        const enumPattern = new RegExp(`enum\\s+${enumName}(?:\\s*<[^>]+>)?\\s*\\{([^}]+)\\}`, 'g');

        for (const doc of docs) {
            const docText = doc.getText();
            let match;
            while ((match = enumPattern.exec(docText)) !== null) {
                const body = match[1];
                const lines = body.split('\n');
                for (const line of lines) {
                    const trimmed = line.trim();
                    if (!trimmed || trimmed.startsWith('//')) continue;
                    
                    const variantMatch = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_]*)/);
                    if (variantMatch) {
                        const variantName = variantMatch[1];
                        if (!items.find(i => i.label === variantName)) {
                            const item = new vscode.CompletionItem(variantName, vscode.CompletionItemKind.EnumMember);
                            if (trimmed.includes('(')) {
                                const payloadStr = trimmed.substring(trimmed.indexOf('(') + 1, trimmed.lastIndexOf(')'));
                                const args = payloadStr.split(',').map(s => s.trim());
                                const snippetParams = args.map((arg, i) => `\${${i + 1}:${arg}}`).join(', ');
                                item.insertText = new vscode.SnippetString(`${variantName}(${snippetParams})`);
                                item.detail = trimmed;
                            }
                            items.push(item);
                        }
                    }
                }
                if (items.length > 0) return items;
            }
        }
        return items;
    }

    private createDeriveCompletion(label: string, doc: string): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Interface);
        item.documentation = new vscode.MarkdownString(doc);
        return item;
    }

    private createDecoratorCompletion(label: string, doc: string): vscode.CompletionItem {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Function);
        // Do not include the @ in the insert text if the user just typed it
        // The prefix is just the word. So we can just use the label. 
        // e.g. label 'log' inserts 'log' after '@'.
        let insertText = label;
        if (label.includes('(')) {
            // e.g. retry(times, backoff) -> retry($1, $2)
            const name = label.substring(0, label.indexOf('('));
            const args = label.substring(label.indexOf('(') + 1, label.indexOf(')')).split(',').map(s => s.trim());
            const snippetParams = args.map((arg, i) => `\${${i + 1}:${arg}}`).join(', ');
            insertText = `${name}(${snippetParams})`;
            item.insertText = new vscode.SnippetString(insertText);
        }
        item.documentation = new vscode.MarkdownString(doc);
        return item;
    }

    private async getMethodsForType(typeName: string): Promise<vscode.CompletionItem[]> {
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

        const items: vscode.CompletionItem[] = [];
        
        // Search for:
        // impl Type { fn Method(self) ... }
        // func (r Type) Method() ...
        // type Type interface { Method() ... }
        
        const implPattern = new RegExp(`^\\s*impl\\s+${typeName}\\s*\\{([^}]+)\\}`, 'gm');
        const goMethodPattern = new RegExp(`^\\s*func\\s*\\(\\s*[a-zA-Z_]\\w*\\s*(?:\\*)?\\s*${typeName}\\s*\\)\\s*([a-zA-Z_]\\w*)\\s*\\(([^)]*)\\)(.*)`, 'gm');
        const interfacePattern = new RegExp(`^\\s*type\\s+${typeName}\\s+interface\\s*\\{([^}]+)\\}`, 'gm');

        for (const doc of docs) {
            const docText = doc.getText();
            
            // GoPlus impl blocks
            let implMatch;
            while ((implMatch = implPattern.exec(docText)) !== null) {
                const body = implMatch[1];
                const fnPattern = /^\s*fn(?:\s+mut)?\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)(.*)/gm;
                let fnMatch;
                while ((fnMatch = fnPattern.exec(body)) !== null) {
                    const methodName = fnMatch[1];
                    const args = fnMatch[2];
                    const retType = fnMatch[3].trim().replace(/^->\s*/, '');
                    
                    const item = new vscode.CompletionItem(methodName, vscode.CompletionItemKind.Method);
                    item.detail = `(method) ${typeName}.${methodName}(${args}) ${retType}`;
                    
                    // Filter out `self` or `mut self` from the arguments to insert
                    const insertArgs = args.split(',')
                        .map(s => s.trim())
                        .filter(s => s && s !== 'self' && s !== 'mut self')
                        .map(s => s.split(':')[0].trim());
                        
                    const snippetParams = insertArgs.map((arg, i) => `\${${i + 1}:${arg}}`).join(', ');
                    item.insertText = new vscode.SnippetString(`${methodName}(${snippetParams})`);
                    
                    items.push(item);
                }
            }

            // Go methods
            let goMatch;
            while ((goMatch = goMethodPattern.exec(docText)) !== null) {
                const methodName = goMatch[1];
                const args = goMatch[2];
                const retType = goMatch[3].trim();
                
                const item = new vscode.CompletionItem(methodName, vscode.CompletionItemKind.Method);
                item.detail = `(method) ${typeName}.${methodName}(${args}) ${retType}`;
                
                const insertArgs = args.split(',')
                    .map(s => s.trim())
                    .filter(s => s)
                    .map(s => s.split(' ')[0].trim());
                    
                const snippetParams = insertArgs.map((arg, i) => `\${${i + 1}:${arg}}`).join(', ');
                item.insertText = new vscode.SnippetString(`${methodName}(${snippetParams})`);
                
                items.push(item);
            }

            // Interface methods
            let intfMatch;
            while ((intfMatch = interfacePattern.exec(docText)) !== null) {
                const body = intfMatch[1];
                // Interface methods look like: MethodName(args) ReturnType
                const intfFnPattern = /^\s*([a-zA-Z_]\w*)\s*\(([^)]*)\)(.*)/gm;
                let fnMatch;
                while ((fnMatch = intfFnPattern.exec(body)) !== null) {
                    const methodName = fnMatch[1];
                    const args = fnMatch[2];
                    const retType = fnMatch[3].trim();
                    
                    const item = new vscode.CompletionItem(methodName, vscode.CompletionItemKind.Method);
                    item.detail = `(interface method) ${typeName}.${methodName}(${args}) ${retType}`;
                    
                    const insertArgs = args.split(',')
                        .map(s => s.trim())
                        .filter(s => s)
                        .map(s => s.split(' ')[0].trim());
                        
                    const snippetParams = insertArgs.map((arg, i) => `\${${i + 1}:${arg}}`).join(', ');
                    item.insertText = new vscode.SnippetString(`${methodName}(${snippetParams})`);
                    
                    items.push(item);
                }
            }
        }

        return items;
    }
}
