use fake::faker::address::raw::*;
use fake::faker::internet::raw::*;
use fake::faker::name::raw::*;
use fake::faker::number;
use fake::faker::phone_number::raw::*;
use fake::Fake;

use ordered_float::OrderedFloat;

use fake::locales::*;
use fakeit::{address, payment, person};
use std::collections::HashSet;
use std::str::FromStr;

use crate::MResult;
use crate::MaskerError;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Category {
    Name,
    FirstName,
    LastName,
    Email,
    Address,
    Ssn,
    City,
    PhoneNumber,
    CreditCard,
    ZipCode,
    PositiveDecimal,
    Inferred,
}

pub type FakeWordPool = Vec<(Category, Vec<String>)>;

const CATEGORIES: &[&'static str] = &[
    "name",
    "first_name",
    "last_name",
    "email",
    "address",
    "ssn",
    "city",
    "phone_number",
    "credit_card",
    "zip_code",
    "positive_decimal"
];


#[derive(Debug)]
pub struct WordClassification {
    pub category: Category,
    pub similar: Vec<String>,
}

impl std::str::FromStr for Category {
    type Err = MaskerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "name" => Ok(Category::Name),
            "first_name" => Ok(Category::FirstName),
            "last_name" => Ok(Category::LastName),
            "email" => Ok(Category::Email),
            "address" => Ok(Category::Address),
            "ssn" => Ok(Category::Ssn),
            "city" => Ok(Category::City),
            "phone_number" => Ok(Category::PhoneNumber),
            "credit_card" => Ok(Category::CreditCard),
            "zip_code" => Ok(Category::ZipCode),
            "positive_decimal" => Ok(Category::PositiveDecimal),
            "inferred" => Ok(Category::Inferred),
            _ => Err(MaskerError::SimilarityError(format!("Invalid category: {}", s).to_string())),
        }
    }
}

pub fn sample_similar_word_for_category(
    word: &str,
    category: Category,
    pool: &FakeWordPool,
    top_words_num: usize,
) -> HashSet<String> {
    let mut coefficients: Vec<(OrderedFloat<f64>, String)> = Vec::new();
    let mut distances: Vec<(OrderedFloat<f64>, String)> = Vec::new();

    let words = {
        if category == Category::Inferred {
            let s = classify_word(word, pool, top_words_num).unwrap().similar;
            return s.iter().cloned().collect();
        } else {
            &pool.iter().find(|x| x.0 == category).unwrap().1
        }
    };
    for fake_word in words {
        let normalized_distance = strsim::normalized_levenshtein(fake_word, word);
        distances.push((OrderedFloat(normalized_distance), fake_word.clone()));
    }

    for distance in distances {
        coefficients.push((distance.0, distance.1.clone()));
    }
    coefficients.sort_by(|a, b| b.0.cmp(&a.0));
    coefficients
        .into_iter()
        .map(|x| x.1)
        .filter(|x| x != word)
        .take(top_words_num)
        .collect()
}

pub fn classify_word(
    word: &str,
    pool: &FakeWordPool,
    top_words_num: usize,
) -> MResult<WordClassification> {
    let mut data: Vec<Vec<String>> = vec![];
    (0..CATEGORIES.len()).for_each(|i| {
        let mut row = vec![];
        let similar_words = sample_similar_word_for_category(
            word,
            Category::from_str(CATEGORIES[i]).unwrap(),
            pool,
            top_words_num,
        );
        row.extend(similar_words);
        data.push(row);
    });

    let mut rows = Vec::new();
    (0..CATEGORIES.len()).for_each(|i| {
        let mut distances = Vec::new();
        for j in 0..data[i].len() {
            let distance = strsim::normalized_levenshtein(&data[i][j], word);
            distances.push((distance, data[i][j].clone()));
        }
        rows.push(distances);
    });

    let mut means = Vec::new();
    for i in 0..CATEGORIES.len() {
        let mut sum: f64 = 0 as f64;
        let mut coefficients: Vec<(OrderedFloat<f64>, String)> = Vec::new();
        for j in 0..rows[i].len() {
            coefficients.push((OrderedFloat(rows[i][j].0), rows[i][j].1.clone()));
            sum += rows[i][j].0;
        }
        let mean: f64 = sum as f64 / rows[i].len() as f64;
        coefficients.sort_by(|a, b| b.0.cmp(&a.0));
        let top_words = coefficients
            .iter()
            .take(top_words_num)
            .cloned()
            .collect::<Vec<(OrderedFloat<f64>, String)>>();

        means.push((mean, CATEGORIES[i], top_words));
    }

    means.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    Ok(WordClassification {
        category: Category::from_str(means[0].1).unwrap(),
        similar: means[0].2.iter().map(|x| x.1.clone()).collect(),
    })
}

pub fn generate_fake_words_pool(category_pool_size: usize) -> FakeWordPool {
    let mut data: FakeWordPool = Vec::new();
    (0..CATEGORIES.len()).for_each(|i| {
        let mut row = vec![];
        for _ in 0..category_pool_size {
            match Category::from_str(CATEGORIES[i]).unwrap() {
                Category::Name => row.push(Name(EN).fake()),
                Category::FirstName => row.push(FirstName(EN).fake()),
                Category::LastName => row.push(LastName(EN).fake()),
                Category::Email => row.push(SafeEmail(EN).fake()),
                Category::Address => row.push(address::street()),
                Category::Ssn => row.push(person::ssn()),
                Category::City => row.push(CityName(EN).fake()),
                Category::PhoneNumber => row.push(PhoneNumber(EN).fake()),
                Category::CreditCard => row.push(payment::credit_card_number()),
                Category::ZipCode => row.push(address::zip()),
                Category::PositiveDecimal => row.push(number::en::NumberWithFormat("####.##").fake()),
                Category::Inferred => {}
            }
        }

        data.push((Category::from_str(CATEGORIES[i]).unwrap(), row));
    });

    data
}

mod tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn test_sample_similar_word_for_category() {
        let generated_pool = generate_fake_words_pool(10000);
        let word = "John".to_string();
        let result =
            sample_similar_word_for_category(&word, Category::FirstName, &generated_pool, 5);
        assert_eq!(result.iter().filter(|x| **x == word).count(), 0);
        assert_eq!(result.len() > 0, true);
    }

    #[test]
    fn test_classify_word() {
        let generated_pool = generate_fake_words_pool(10000);
        let word = "susan@gmail.com".to_string();
        let result = classify_word(&word, &generated_pool, 5);
        assert_eq!(result.is_err(), false);
        assert_eq!(result.as_ref().unwrap().category, Category::Email);
        assert_eq!(
            result
                .as_ref()
                .unwrap()
                .similar
                .iter()
                .filter(|x| **x == word)
                .count(),
            0
        );
        assert_eq!(result.as_ref().unwrap().similar.len() > 0, true);
    }
}
