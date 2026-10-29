pub fn is_probable_url(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("http://") || t.starts_with("https://"))
        && t.len() > 12
        && !t.contains('\n')
        && !t.contains('\r')
        && !t.contains(' ')
        && t.contains('.')
}
