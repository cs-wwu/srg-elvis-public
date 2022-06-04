// A little utility function to print the type of a variable.
// Useful for debugging
pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
