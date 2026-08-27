/// Strip control characters (except newline) and trim. Prevents users from
/// smuggling unexpected bytes into TTS / the LLM prompt.
pub fn sanitize_text(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect::<String>()
        .trim()
        .to_string()
}
