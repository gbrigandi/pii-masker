#![allow(dead_code)]

extern crate pii_masker_derive;

use pii_masker_derive::PIIMask;

//
// Define the Course struct
#[derive(Debug, PartialEq)]
struct Course {
    name: String,
    credits: u32,
}

// Define the Department struct
#[derive(Debug, PIIMask)]
struct Department {
//    #[pii_mask(ssn)]
    #[pii_mask(faker = "first_name")]
    name: String,
    courses: Vec<Course>,
}

impl Department {
    // Adds a new course to the department
    fn add_course(&mut self, course: Course) {
        self.courses.push(course);
    }

    // List all courses in the department
    fn list_courses(&self) -> Vec<String> {
        self.courses.iter().map(|c| c.name.clone()).collect()
    }
}

// Define the University struct
#[derive(Debug)]
struct University {
    name: String,
    departments: Vec<Department>,
}

impl University {
    // Adds a new department to the university
    fn add_department(&mut self, department: Department) {
        self.departments.push(department);
    }

    // List all departments in the university
    fn list_departments(&self) -> Vec<String> {
        self.departments.iter().map(|d| d.name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_add_course() {
        let mut dept = Department {
            name: String::from("Computer Science"),
            courses: vec![],
        };
        let course = Course {
            name: String::from("Introduction to Rust"),
            credits: 3,
        };
        dept.add_course(course);

        assert_eq!(dept.courses.len(), 1);
        assert_eq!(dept.courses[0].name, "Introduction to Rust");
    }

    #[test]
    fn test_list_courses() {
        let dept = Department {
            name: String::from("History"),
            courses: vec![
                Course {
                    name: String::from("World History"),
                    credits: 3,
                },
            ],
        };
        let course_names = dept.list_courses();
        assert_eq!(course_names, vec![String::from("World History")]);
    }

    #[test]
    fn test_add_department() {
        let mut uni = University {
            name: String::from("Generic University"),
            departments: vec![],
        };
        let dept = Department {
            name: String::from("Engineering"),
            courses: vec![],
        };
        uni.add_department(dept);

        assert_eq!(uni.departments.len(), 1);
        assert_eq!(uni.departments[0].name, "Engineering");
    }

    #[test]
    fn test_list_departments() {
        let uni = University {
            name: String::from("Generic University"),
            departments: vec![
                Department {
                    name: String::from("Mathematics"),
                    courses: vec![],
                },
            ],
        };
        let dept_names = uni.list_departments();
        assert_eq!(dept_names, vec![String::from("Mathematics")]);
    }
}

fn main() {
    // Example usage
    let mut uni = University {
        name: String::from("Example University"),
        departments: Vec::new(),
    };

    let mut cs_dept = Department {
        name: String::from("Computer Science"),
        courses: Vec::new(),
    };

    cs_dept.add_course(Course {
        name: String::from("Intro to Programming"),
        credits: 4,
    });

    uni.add_department(cs_dept);

    println!("{:?}", uni);
}
