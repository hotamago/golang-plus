use super::*;

pub(super) fn render_unit_diagnostics(items: &[UnitDiagnostics]) -> String {
    items
        .iter()
        .flat_map(|item| {
            item.diagnostics
                .iter()
                .map(|diag| diag.render(&item.path.display().to_string(), &item.source))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn render_unit_diagnostics_json(items: &[UnitDiagnostics]) -> String {
    let rendered = items
        .iter()
        .flat_map(|item| {
            item.diagnostics
                .iter()
                .map(|diag| diag.render_json(&item.path.display().to_string(), &item.source))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"diagnostics":[{rendered}]}}"#)
}

pub(super) fn render_unit_diagnostics_with_format(
    items: &[UnitDiagnostics],
    format: DiagnosticFormat,
) -> String {
    match format {
        DiagnosticFormat::Human => render_unit_diagnostics(items),
        DiagnosticFormat::Json => render_unit_diagnostics_json(items),
    }
}
