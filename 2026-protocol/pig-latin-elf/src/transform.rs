//! Pig Latin transformation logic

/// Check if a character is a vowel
fn is_vowel(c: char) -> bool {
    matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u')
}

/// Transform a single word to pig latin
/// - Words starting with vowel: add "way" (apple → appleway)
/// - Words starting with consonant: move consonant(s) to end + "ay" (hello → ellohay)
fn word_to_pig_latin(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }

    // Find first letter
    let first_letter_pos = word.chars().position(|c| c.is_alphabetic());
    if first_letter_pos.is_none() {
        // No letters in word
        return word.to_string();
    }

    let first_letter_pos = first_letter_pos.unwrap();
    let first_char = word.chars().nth(first_letter_pos).unwrap();

    // Check if starts with vowel
    if is_vowel(first_char) {
        format!("{}way", word)
    } else {
        // Find the first vowel position (consonant cluster)
        let vowel_pos = word.chars()
            .skip(first_letter_pos)
            .position(|c| is_vowel(c));

        match vowel_pos {
            Some(pos) => {
                let split_pos = first_letter_pos + pos;
                let prefix = &word[..first_letter_pos];
                let consonants = &word[first_letter_pos..split_pos];
                let rest = &word[split_pos..];

                // Handle capitalization
                if consonants.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    let consonants_lower = consonants.to_lowercase();
                    let mut rest_chars = rest.chars();
                    let rest_capitalized: String = if let Some(first) = rest_chars.next() {
                        first.to_uppercase().chain(rest_chars).collect()
                    } else {
                        String::new()
                    };
                    format!("{}{}{}ay", prefix, rest_capitalized, consonants_lower)
                } else {
                    format!("{}{}{}ay", prefix, rest, consonants)
                }
            }
            None => {
                // No vowels, just add "ay"
                format!("{}ay", word)
            }
        }
    }
}

/// Transform a string to pig latin
/// Preserves non-alphabetic characters and word boundaries
pub fn to_pig_latin(text: &str) -> String {
    let mut result = String::new();
    let mut current_word = String::new();

    for c in text.chars() {
        if c.is_alphabetic() {
            current_word.push(c);
        } else {
            if !current_word.is_empty() {
                result.push_str(&word_to_pig_latin(&current_word));
                current_word.clear();
            }
            result.push(c);
        }
    }

    // Handle last word
    if !current_word.is_empty() {
        result.push_str(&word_to_pig_latin(&current_word));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consonant_start() {
        assert_eq!(to_pig_latin("hello"), "ellohay");
        assert_eq!(to_pig_latin("world"), "orldway");
    }

    #[test]
    fn test_vowel_start() {
        assert_eq!(to_pig_latin("apple"), "appleway");
        assert_eq!(to_pig_latin("is"), "isway");
    }

    #[test]
    fn test_consonant_cluster() {
        assert_eq!(to_pig_latin("string"), "ingstray");
        assert_eq!(to_pig_latin("chrome"), "omechray");
    }

    #[test]
    fn test_sentence() {
        assert_eq!(to_pig_latin("hello world"), "ellohay orldway");
    }

    #[test]
    fn test_capitalization() {
        assert_eq!(to_pig_latin("Hello"), "Ellohay");
    }

    #[test]
    fn test_punctuation() {
        assert_eq!(to_pig_latin("hello!"), "ellohay!");
        assert_eq!(to_pig_latin("hello, world!"), "ellohay, orldway!");
    }
}
