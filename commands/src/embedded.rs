//! `rules.md` and the templates, built in from `plugins/tuq/` at compile time (not copied), so the programs and the
//! skills they ship with always match.

pub const RULES: &str = include_str!("../../plugins/tuq/rules.md");

pub const TEMPLATE_DECISION: &str = include_str!("../../plugins/tuq/templates/decision.md");
pub const TEMPLATE_README: &str = include_str!("../../plugins/tuq/templates/README.md");
pub const TEMPLATE_TECH_SPEC: &str = include_str!("../../plugins/tuq/templates/tech-spec.md");
pub const TEMPLATE_PLAN: &str = include_str!("../../plugins/tuq/templates/plan.md");
pub const TEMPLATE_ORCHESTRATOR: &str = include_str!("../../plugins/tuq/templates/orchestrator.md");

/// A template by the name `tuq template` takes.
pub fn template(name: &str) -> Option<&'static str> {
    Some(match name {
        "decision" => TEMPLATE_DECISION,
        "readme" => TEMPLATE_README,
        "tech-spec" => TEMPLATE_TECH_SPEC,
        "plan" => TEMPLATE_PLAN,
        "orchestrator" => TEMPLATE_ORCHESTRATOR,
        _ => return None,
    })
}
