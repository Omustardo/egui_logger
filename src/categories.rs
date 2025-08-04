// Trait to handle different category input types
pub trait IntoCategories {
    fn into_categories(self) -> Vec<String>;
}

// Implementation for vectors
impl<T: ToString> IntoCategories for Vec<T> {
    fn into_categories(self) -> Vec<String> {
        self.into_iter().map(|c| c.to_string()).collect()
    }
}

// Implementation for arrays
impl<T: ToString, const N: usize> IntoCategories for [T; N] {
    fn into_categories(self) -> Vec<String> {
        self.into_iter().map(|c| c.to_string()).collect()
    }
}

// Implementation for slices
impl<T: ToString> IntoCategories for &[T] {
    fn into_categories(self) -> Vec<String> {
        self.iter().map(|c| c.to_string()).collect()
    }
}

// Implementation for string types
impl IntoCategories for &str {
    fn into_categories(self) -> Vec<String> {
        vec![self.to_string()]
    }
}
impl IntoCategories for String {
    fn into_categories(self) -> Vec<String> {
        vec![self]
    }
}
impl IntoCategories for &String {
    fn into_categories(self) -> Vec<String> {
        vec![self.clone()]
    }
}