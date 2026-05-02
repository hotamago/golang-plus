"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.GoplusCompletionProvider = void 0;
const vscode = __importStar(require("vscode"));
class GoplusCompletionProvider {
    provideCompletionItems(document, position, token, context) {
        const linePrefix = document.lineAt(position).text.substring(0, position.character);
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
        const items = [];
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
        const seen = new Set();
        while ((match = definitionRegex.exec(docText)) !== null) {
            const name = match[1];
            if (!seen.has(name)) {
                seen.add(name);
                items.push(new vscode.CompletionItem(name, vscode.CompletionItemKind.Reference));
            }
        }
        return items;
    }
    createDeriveCompletion(label, doc) {
        const item = new vscode.CompletionItem(label, vscode.CompletionItemKind.Interface);
        item.documentation = new vscode.MarkdownString(doc);
        return item;
    }
    createDecoratorCompletion(label, doc) {
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
}
exports.GoplusCompletionProvider = GoplusCompletionProvider;
//# sourceMappingURL=completion.js.map