#[derive(Debug,PIIMask)]
struct Student {
    #[pii_mask(first_name)]
    first_name: String,
    #[pii_mask(last_name)]
    last_name: String,
    #[pii_mask(ssn)]
    ssn: String,
    #[pii_mask(inferred)]
    mobile: String
}

#[cfg(test)]
mod tests {
  user super::*;

  #[test]
  fn test_lookup_student() {
    let expected_student = Student {
        first_name: "John",
        last_name: "Doe",
        ssn: "123-45-6789",
        mobile: "310-333-2132"
    };

    assert_eq!(find_student(100), expected_student);

  }
}
