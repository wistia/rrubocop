fn normalized(text: &str) -> String {
    text.to_lowercase().replace('@', "")
}
fn jaro(first: &str, second: &str) -> f64 {
    let mut a: Vec<_> = first.chars().collect();
    let mut b: Vec<_> = second.chars().collect();
    if a.len() > b.len() {
        std::mem::swap(&mut a, &mut b);
    }
    let distance = if b.len() > 3 { b.len() / 2 - 1 } else { 0 };
    let mut aa = vec![false; a.len()];
    let mut bb = vec![false; b.len()];
    let mut matched = 0;
    for i in 0..a.len() {
        for j in i.saturating_sub(distance)..=(i + distance).min(b.len().saturating_sub(1)) {
            if !bb[j] && a[i] == b[j] {
                aa[i] = true;
                bb[j] = true;
                matched += 1;
                break;
            }
        }
    }
    if matched == 0 {
        return 0.0;
    }
    let transposed = a
        .iter()
        .zip(&aa)
        .filter(|(_, flag)| **flag)
        .map(|(ch, _)| ch)
        .zip(
            b.iter()
                .zip(&bb)
                .filter(|(_, flag)| **flag)
                .map(|(ch, _)| ch),
        )
        .filter(|(a, b)| a != b)
        .count()
        / 2;
    let m = matched as f64;
    let score = (m / a.len() as f64 + m / b.len() as f64 + (m - transposed as f64) / m) / 3.0;
    if score > 0.7 {
        score
            + a.iter().zip(&b).take(4).take_while(|(a, b)| a == b).count() as f64
                * 0.1
                * (1.0 - score)
    } else {
        score
    }
}
fn levenshtein(first: &str, second: &str) -> usize {
    let a: Vec<_> = first.chars().collect();
    let b: Vec<_> = second.chars().collect();
    let mut previous: Vec<_> = (0..=b.len()).collect();
    for (i, ch) in a.iter().enumerate() {
        let mut row = vec![i + 1];
        for (j, other) in b.iter().enumerate() {
            row.push(
                (row[j] + 1)
                    .min(previous[j + 1] + 1)
                    .min(previous[j] + usize::from(ch != other)),
            );
        }
        previous = row;
    }
    previous[b.len()]
}
pub(super) fn suggestion<'a>(target: &str, names: &'a [String]) -> Option<&'a str> {
    let input = normalized(target);
    let len = input.chars().count();
    let threshold = if len > 3 { 0.834 } else { 0.77 };
    let mut words: Vec<_> = names
        .iter()
        .filter(|word| word.as_str() != target && jaro(&normalized(word), &input) >= threshold)
        .collect();
    words.sort_by(|a, b| jaro(a, &input).total_cmp(&jaro(b, &input)));
    words.reverse();
    let edit_threshold = len.div_ceil(4);
    words
        .iter()
        .find(|word| levenshtein(&normalized(word), &input) <= edit_threshold)
        .or_else(|| {
            words.iter().find(|word| {
                levenshtein(&normalized(word), &input) < len.min(normalized(word).chars().count())
            })
        })
        .map(|word| word.as_str())
}
