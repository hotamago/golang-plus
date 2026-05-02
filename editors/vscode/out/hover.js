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
exports.GoplusHoverProvider = void 0;
const vscode = __importStar(require("vscode"));
const DECORATOR_DOCS = {
    'log': {
        signature: '@log',
        description: 'Wraps the function with automatic entry/exit logging. Prints function name and arguments on call, and return values on completion.',
        example: '@log\nfn readName() -> string! {\n    return "goplus"\n}',
    },
    'retry': {
        signature: '@retry(times: int[, backoff_ms: int])',
        description: 'Retries the function up to `times` attempts on error. Optional `backoff_ms` adds a delay between retries.',
        example: '@retry(3, 100)\nfn fetchData() -> string! {\n    return http.Get(url)?\n}',
    },
    'memoize': {
        signature: '@memoize',
        description: 'Caches function results by arguments. Subsequent calls with the same arguments return the cached result without re-executing the function body.',
        example: '@memoize\nfn fibonacci(n: int) -> int {\n    if n <= 1 { return n }\n    return fibonacci(n-1) + fibonacci(n-2)\n}',
    },
    'derive': {
        signature: '@derive(Kind1, Kind2, ...)',
        description: 'Generates method implementations for structs and enums at compile time.',
        example: '@derive(String, Debug, Equal)\nstruct User {\n    Name: string\n    Age: int\n}',
    },
};
const DERIVE_KIND_DOCS = {
    'String': '**String** — generates `func (t Type) String() string` for human-readable output via `fmt.Println`.',
    'Debug': '**Debug** — generates `func (t Type) Debug() string` with detailed field-level debug representation.',
    'Equal': '**Equal** — generates `func (t Type) Equal(other Type) bool` for structural equality comparison.',
    'JsonMarshal': '**JsonMarshal** — generates `func (t Type) MarshalJSON() ([]byte, error)` for JSON serialization.',
    'JsonUnmarshal': '**JsonUnmarshal** — generates `func (t *Type) UnmarshalJSON(data []byte) error` for JSON deserialization.',
};
class GoplusHoverProvider {
    async findDefinitionAndHover(word) {
        // Simple regex search across workspace .gp files
        const uris = await vscode.workspace.findFiles('**/*.gp', '**/node_modules/**');
        const docs = new Set(vscode.workspace.textDocuments.filter(d => d.languageId === 'goplus'));
        for (const uri of uris) {
            const openDoc = vscode.workspace.textDocuments.find(d => d.uri.fsPath === uri.fsPath);
            if (!openDoc) {
                try {
                    const doc = await vscode.workspace.openTextDocument(uri);
                    docs.add(doc);
                }
                catch {
                    // Ignore errors opening files
                }
            }
        }
        const pattern = new RegExp(`^\\s*(?:fn(?:\\s+mut)?|struct|enum|type|const|var)\\s+${word}\\b`);
        const methodPattern = new RegExp(`^\\s*fn(?:\\s+mut)?\\s+${word}\\b`);
        for (const doc of docs) {
            for (let i = 0; i < doc.lineCount; i++) {
                const text = doc.lineAt(i).text;
                if (pattern.test(text) || methodPattern.test(text)) {
                    // Found a definition! Extract the signature
                    const signature = text.trim().replace(/ \{$/, '');
                    // Look for preceding comments
                    const comments = [];
                    let j = i - 1;
                    while (j >= 0) {
                        const prevLine = doc.lineAt(j).text.trim();
                        if (prevLine.startsWith('//')) {
                            comments.unshift(prevLine.substring(2).trim());
                            j--;
                        }
                        else {
                            break;
                        }
                    }
                    const md = new vscode.MarkdownString();
                    md.appendCodeblock(signature, 'gp');
                    if (comments.length > 0) {
                        md.appendMarkdown('\n---\n' + comments.join('  \n'));
                    }
                    return new vscode.Hover(md);
                }
            }
        }
        return undefined;
    }
    async provideHover(document, position, _token) {
        const lineText = document.lineAt(position.line).text;
        const wordRange = document.getWordRangeAtPosition(position, /[@a-zA-Z_!?:][a-zA-Z0-9_]*/);
        if (!wordRange) {
            return undefined;
        }
        const word = document.getText(wordRange);
        const charAtPos = lineText[position.character] || '';
        // Decorator hover: @name
        if (word.startsWith('@')) {
            const decoratorName = word.substring(1);
            return this.hoverDecorator(decoratorName, lineText);
        }
        // Derive kind hover inside @derive(...)
        if (DERIVE_KIND_DOCS[word] && lineText.includes('@derive')) {
            const md = new vscode.MarkdownString();
            md.appendMarkdown(`### \`@derive(${word})\`\n\n`);
            md.appendMarkdown(DERIVE_KIND_DOCS[word]);
            return new vscode.Hover(md);
        }
        // ? operator hover
        if (charAtPos === '?') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `?` — Try Operator\n\n');
            md.appendMarkdown('Unwraps the result value or returns the error to the caller.\n\n');
            md.appendMarkdown('**Generated Go equivalent:**\n');
            md.appendCodeblock('val, err := expr()\nif err != nil {\n    return ..., err\n}', 'go');
            return new vscode.Hover(md);
        }
        // ! error return type hover
        if (charAtPos === '!') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `!` — Error Return Type\n\n');
            md.appendMarkdown('Indicates the function returns an error alongside its value.\n\n');
            md.appendMarkdown('| GoPlus | Generated Go |\n|--------|-------------|\n');
            md.appendMarkdown('| `-> T!` | `(T, error)` |\n');
            md.appendMarkdown('| `-> !` | `error` (main becomes a wrapper) |\n');
            return new vscode.Hover(md);
        }
        // :: namespace operator
        if (word === '::' || lineText.substring(position.character, position.character + 2) === '::') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `::` — Enum Variant Access\n\n');
            md.appendMarkdown('Accesses a variant of a tagged or simple enum.\n\n');
            md.appendMarkdown('```gp\nStatus::Running\nResult<string>::Ok("value")\n```');
            return new vscode.Hover(md);
        }
        // self keyword
        if (word === 'self') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `self` — Receiver\n\n');
            md.appendMarkdown('Refers to the current instance of the `impl` target type.\n\n');
            md.appendMarkdown('- `fn method(self)` → value receiver `func (s Type)`\n');
            md.appendMarkdown('- `fn mut method(self)` → pointer receiver `func (s *Type)`\n');
            return new vscode.Hover(md);
        }
        // match keyword
        if (word === 'match') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `match` — Pattern Matching\n\n');
            md.appendMarkdown('Exhaustively matches enum variants. The compiler checks that all variants are covered.\n\n');
            md.appendMarkdown('```gp\nmatch result {\n    Ok(value) => "success: " + value,\n    Err(reason) => "error: " + reason,\n}\n```\n\n');
            md.appendMarkdown('Compiles to a Go `switch` statement.');
            return new vscode.Hover(md);
        }
        // fn keyword
        if (word === 'fn') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `fn` — Function Declaration\n\n');
            md.appendMarkdown('Declares a function with GoPlus syntax.\n\n');
            md.appendMarkdown('```gp\nfn name(param: Type) -> ReturnType {\n    // body\n}\n```\n\n');
            md.appendMarkdown('Inside `impl` blocks, use `fn mut` for pointer receivers.');
            return new vscode.Hover(md);
        }
        // enum keyword
        if (word === 'enum') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `enum` — Enum Declaration\n\n');
            md.appendMarkdown('Declares a sum type with simple or tagged (payload) variants.\n\n');
            md.appendMarkdown('```gp\nenum Result<T> {\n    Ok(T)\n    Err(string)\n}\n```\n\n');
            md.appendMarkdown('Use `@derive(String)` to add `String()` method.');
            return new vscode.Hover(md);
        }
        // struct keyword
        if (word === 'struct') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `struct` — Struct Declaration\n\n');
            md.appendMarkdown('Declares a product type with named fields.\n\n');
            md.appendMarkdown('```gp\nstruct User {\n    Name: string\n    Age: int\n}\n```');
            return new vscode.Hover(md);
        }
        // impl keyword
        if (word === 'impl') {
            const md = new vscode.MarkdownString();
            md.appendMarkdown('### `impl` — Method Implementation Block\n\n');
            md.appendMarkdown('Groups methods for a struct or enum type.\n\n');
            md.appendMarkdown('```gp\nimpl User {\n    fn Greet(self) -> string {\n        return "Hi, " + self.Name\n    }\n}\n```');
            return new vscode.Hover(md);
        }
        // Fallback: search for definition to show signature and comment
        if (/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(word)) {
            return await this.findDefinitionAndHover(word);
        }
        return undefined;
    }
    hoverDecorator(name, lineText) {
        const doc = DECORATOR_DOCS[name];
        if (!doc) {
            // Unknown/custom decorator
            const md = new vscode.MarkdownString();
            md.appendMarkdown(`### \`@${name}\` — Custom Decorator\n\n`);
            md.appendMarkdown('A user-defined decorator function that wraps the target function.\n\n');
            md.appendMarkdown('Custom decorators take `next` (the original function) and optional arguments, returning a function with the same signature.');
            return new vscode.Hover(md);
        }
        const md = new vscode.MarkdownString();
        md.appendMarkdown(`### \`${doc.signature}\`\n\n`);
        md.appendMarkdown(`${doc.description}\n\n`);
        md.appendMarkdown('**Example:**\n');
        md.appendCodeblock(doc.example, 'gp');
        // For @derive, also list available kinds
        if (name === 'derive') {
            md.appendMarkdown('\n**Available derive kinds:**\n\n');
            for (const [kind, desc] of Object.entries(DERIVE_KIND_DOCS)) {
                md.appendMarkdown(`- ${desc}\n`);
            }
        }
        return new vscode.Hover(md);
    }
}
exports.GoplusHoverProvider = GoplusHoverProvider;
//# sourceMappingURL=hover.js.map