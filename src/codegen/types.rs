use super::*;

pub(super) struct FnSignature {
    pub(super) name: String,
    pub(super) type_params: Vec<String>,
    pub(super) params: Vec<ParamDecl>,
    pub(super) ret: ReturnType,
    pub(super) receiver: Option<(String, String)>,
}

pub(super) fn render_signature(sig: &FnSignature, go_name: &str) -> String {
    let receiver = sig
        .receiver
        .as_ref()
        .map(|(name, ty)| format!("({} {}) ", name, ty))
        .unwrap_or_default();
    let type_params = if sig.receiver.is_some() {
        String::new()
    } else {
        render_type_params_with_any(&sig.type_params)
    };
    let params = sig
        .params
        .iter()
        .map(|param| format!("{} {}", param.name, render_type_ref(&param.ty)))
        .collect::<Vec<_>>()
        .join(", ");
    let ret = render_return_type(&sig.ret);
    format!(
        "func {}{}{}({}){}",
        receiver, go_name, type_params, params, ret
    )
}

pub(super) fn call_expr(sig: &FnSignature, target_name: &str) -> String {
    call_value_expr(sig, &call_target_expr(sig, target_name))
}

pub(super) fn call_target_expr(sig: &FnSignature, target_name: &str) -> String {
    if sig.receiver.is_some() {
        format!("self.{}", target_name)
    } else {
        target_name.to_string()
    }
}

pub(super) fn call_value_expr(sig: &FnSignature, callee_expr: &str) -> String {
    let args = sig
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({})", callee_expr, args)
}

pub(super) fn render_return_type(ret: &ReturnType) -> String {
    match ret {
        ReturnType::Void => String::new(),
        ReturnType::Type(ty) => format!(" {}", render_type_ref(ty)),
        ReturnType::ErrorOnly => " error".to_string(),
        ReturnType::TypeWithError(ty) => format!(" ({}, error)", render_type_ref(ty)),
    }
}

pub(super) fn render_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        return String::new();
    }
    format!("[{}]", type_params.join(", "))
}

pub(super) fn render_type_params_with_any(type_params: &[String]) -> String {
    if type_params.is_empty() {
        return String::new();
    }
    let params = type_params
        .iter()
        .map(|param| format!("{} any", param))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{}]", params)
}

pub(super) fn render_type_args(type_params: &[String]) -> String {
    if type_params.is_empty() {
        return String::new();
    }
    format!("[{}]", type_params.join(", "))
}

pub(super) fn render_type_ref(ty: &TypeRef) -> String {
    normalize_go_type(&ty.raw)
}

pub(super) fn normalize_go_type(raw: &str) -> String {
    rewrite_generic_type_syntax(&rewrite_fn_syntax(raw.trim()))
}

pub(super) fn rewrite_generic_type_syntax(input: &str) -> String {
    let mut out = String::new();
    let bytes = input.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if is_ident_start(bytes[i]) {
            let ident_start = i;
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            out.push_str(&input[ident_start..i]);

            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len()
                && bytes[j] == b'<'
                && let Some(close) = find_matching_angle(input, j)
            {
                out.push('[');
                out.push_str(&rewrite_generic_type_syntax(&input[j + 1..close]));
                out.push(']');
                i = close + 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

fn find_matching_angle(input: &str, open_idx: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    for (idx, b) in bytes.iter().enumerate().skip(open_idx) {
        match *b {
            b'(' => paren += 1,
            b')' => paren = paren.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b'<' if paren == 0 && bracket == 0 => depth += 1,
            b'>' if paren == 0 && bracket == 0 => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

pub(super) fn rewrite_fn_syntax(input: &str) -> String {
    let mut out = input.to_string();
    loop {
        let (next, changed) = rewrite_fn_syntax_once(&out);
        out = next;
        if !changed {
            return out;
        }
    }
}

pub(super) fn rewrite_fn_syntax_once(input: &str) -> (String, bool) {
    let bytes = input.as_bytes();
    if bytes.len() < 2 {
        return (input.to_string(), false);
    }

    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'f'
            && bytes[i + 1] == b'n'
            && is_ident_boundary_before(bytes, i)
            && is_ident_boundary_after(bytes, i + 2)
        {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j >= bytes.len() || bytes[j] != b'(' {
                i += 1;
                continue;
            }

            let Some(params_end) = find_matching_paren(input, j) else {
                i += 1;
                continue;
            };
            let params = &input[j + 1..params_end];

            let mut k = params_end + 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k + 1 >= bytes.len() || &input[k..k + 2] != "->" {
                i += 1;
                continue;
            }
            k += 2;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            let ret_end = find_type_segment_end(input, k);
            let ret_raw = &input[k..ret_end];

            let rendered_params = render_fn_params(params);
            let rendered_ret = render_fn_return(ret_raw);
            let replacement = format!("func({}) {}", rendered_params, rendered_ret);

            let mut out = String::new();
            out.push_str(&input[..i]);
            out.push_str(&replacement);
            out.push_str(&input[ret_end..]);
            return (out, true);
        }
        i += 1;
    }

    (input.to_string(), false)
}

pub(super) fn render_fn_params(raw: &str) -> String {
    split_top_level(raw, ',')
        .into_iter()
        .filter_map(|segment| {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Some(colon) = find_top_level_char(trimmed, ':') {
                let name = trimmed[..colon].trim();
                let ty = trimmed[colon + 1..].trim();
                if name.is_empty() {
                    Some(normalize_go_type(ty))
                } else {
                    Some(format!("{} {}", name, normalize_go_type(ty)))
                }
            } else {
                Some(normalize_go_type(trimmed))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn render_fn_return(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed == "!" {
        return "error".to_string();
    }
    if let Some(value_ty) = trimmed.strip_suffix('!') {
        let value_ty = value_ty.trim();
        if value_ty.is_empty() {
            return "error".to_string();
        }
        return format!("({}, error)", normalize_go_type(value_ty));
    }
    normalize_go_type(trimmed)
}

pub(super) fn split_top_level(input: &str, delim: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut angle = 0usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '{' => brace += 1,
            '}' => brace = brace.saturating_sub(1),
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            _ => {}
        }
        if ch == delim && paren == 0 && bracket == 0 && brace == 0 && angle == 0 {
            parts.push(&input[start..idx]);
            start = idx + ch.len_utf8();
        }
    }
    parts.push(&input[start..]);
    parts
}

pub(super) fn find_top_level_char(input: &str, needle: char) -> Option<usize> {
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut angle = 0usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => paren += 1,
            ')' => paren = paren.saturating_sub(1),
            '[' => bracket += 1,
            ']' => bracket = bracket.saturating_sub(1),
            '{' => brace += 1,
            '}' => brace = brace.saturating_sub(1),
            '<' => angle += 1,
            '>' => angle = angle.saturating_sub(1),
            _ => {}
        }
        if ch == needle && paren == 0 && bracket == 0 && brace == 0 && angle == 0 {
            return Some(idx);
        }
    }
    None
}

pub(super) fn find_matching_paren(input: &str, open_idx: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    if open_idx >= bytes.len() || bytes[open_idx] != b'(' {
        return None;
    }
    let mut depth = 0usize;
    for (idx, b) in bytes.iter().enumerate().skip(open_idx) {
        if *b == b'(' {
            depth += 1;
        } else if *b == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

pub(super) fn find_type_segment_end(input: &str, start: usize) -> usize {
    let bytes = input.as_bytes();
    let mut i = start;
    let mut paren = 0usize;
    let mut bracket = 0usize;
    let mut brace = 0usize;
    let mut angle = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'(' => paren += 1,
            b')' => {
                if paren > 0 {
                    paren -= 1;
                } else if bracket == 0 && brace == 0 && angle == 0 {
                    break;
                }
            }
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            b'{' => {
                if paren == 0 && bracket == 0 && brace == 0 && angle == 0 {
                    break;
                }
                brace += 1;
            }
            b'}' => brace = brace.saturating_sub(1),
            b'<' => angle += 1,
            b'>' => angle = angle.saturating_sub(1),
            b',' | b';' | b'\n' | b'\r' => {
                if paren == 0 && bracket == 0 && brace == 0 && angle == 0 {
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }

    i
}

pub(super) fn is_ident_boundary_before(bytes: &[u8], idx: usize) -> bool {
    if idx == 0 {
        return true;
    }
    let b = bytes[idx - 1];
    !b.is_ascii_alphanumeric() && b != b'_'
}

pub(super) fn is_ident_boundary_after(bytes: &[u8], idx: usize) -> bool {
    if idx >= bytes.len() {
        return true;
    }
    let b = bytes[idx];
    !b.is_ascii_alphanumeric() && b != b'_'
}

pub(super) fn lower_ident(input: &str) -> String {
    let mut chars = input.chars();
    if let Some(first) = chars.next() {
        format!(
            "{}{}",
            first.to_ascii_lowercase(),
            chars.collect::<String>()
        )
    } else {
        String::new()
    }
}

pub(super) fn zero_value_expr(ty: &str) -> String {
    let normalized = normalize_go_type(ty);
    let trimmed = normalized.trim();
    if trimmed == "string" {
        return "\"\"".to_string();
    }
    if trimmed == "bool" {
        return "false".to_string();
    }
    if matches!(
        trimmed,
        "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "float32"
            | "float64"
            | "byte"
            | "rune"
    ) {
        return "0".to_string();
    }
    if trimmed.starts_with('*')
        || trimmed.starts_with("[]")
        || trimmed.starts_with("map[")
        || trimmed.starts_with("func(")
        || trimmed == "error"
    {
        return "nil".to_string();
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && trimmed
            .chars()
            .next()
            .map(|ch| ch.is_ascii_uppercase())
            .unwrap_or(false)
    {
        return format!("{}{{}}", trimmed);
    }
    format!("{}{{}}", trimmed)
}

pub(super) fn is_error_ctor(expr: &str) -> bool {
    expr.trim().starts_with("error(")
}

pub(super) fn is_direct_error_expr(expr: &str) -> bool {
    let trimmed = expr.trim();
    trimmed.ends_with(".Error") || is_error_value_expr(trimmed)
}

pub(super) fn is_error_value_expr(expr: &str) -> bool {
    let trimmed = expr.trim();
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return false;
    }
    trimmed == "err" || trimmed.ends_with("Err") || trimmed.ends_with("Error")
}

pub(super) fn map_error_ctor(expr: &str, errors_name: &str) -> String {
    expr.replacen("error(", &format!("{errors_name}.New("), 1)
}

pub(super) fn tabs(indent: usize) -> String {
    "\t".repeat(indent)
}
