use std::io;

fn main() {
    // prompt the user to enter the operation, either NOT, AND, or OR.
    println!("Enter the operation to perform: NOT, AND, or OR");
    let mut operation = String::new();
    let mut num1 = String::new();
    let mut num2 = String::new();

    io::stdin()
        .read_line(&mut operation)
        .expect("Failed to read line");
    let operation: String = operation.trim().parse().expect("Please type a number!");

    if operation == "NOT" {
        println!("Enter one three digit binary number to use NOT on.");
        io::stdin()
            .read_line(&mut num1)
            .expect("Failed to read line");
        let num1: String = num1.trim().parse().expect("Please type a number!");
        println!("> {}", three_bit_NOT(&num1));

    } else if operation == "OR" {
        println!("Enter two three digit binary numbers to use OR on.");
        io::stdin()
            .read_line(&mut num1)
            .expect("Failed to read line");
        let num1: String = num1.trim().parse().expect("Please type a number!");
        io::stdin()
            .read_line(&mut num2)
            .expect("Failed to read line");
        let num2: String = num1.trim().parse().expect("Please type a number!");
        println!("> {}", three_bit_OR(&num1, &num2));

    } else if operation == "AND" {
        println!("Enter two three digit binary numbers to use AND on.");
        io::stdin()
            .read_line(&mut num1)
            .expect("Failed to read line");
        let num1: String = num1.trim().parse().expect("Please type a number!");
        io::stdin()
            .read_line(&mut num2)
            .expect("Failed to read line");
        let num1: String = num1.trim().parse().expect("Please type a number!");
        println!("> {}", three_bit_AND(&num1, &num2));

    } else {
        println!("Wrong operation")
    }
}

fn one_bit_NOT(single_char: char) -> char {
    if single_char == '0' {
        return '1';
    } else if single_char == '1' {
        return '0';
    } else {
        return '2';
    }
}

//works with three or more characters in arg
fn three_bit_NOT(three_bit: &str) -> String {
    let mut result = String::new();
    for c in three_bit.chars() {
        if c == '0' {
            result.push('1');
        } else if c == '1' {
            result.push('0');
        } else {
            result.push('2');
        }
    }
    return result;
}

fn one_bit_OR(a:char, b:char) -> char {
    if a == '1' {
        return '1';
    } else if b == '1' {
        return '1';
    } else {
        return '0';
    }
}

// there is something wrong with this, for example: 100 and 001 should return 101, but returns 100 instead
fn three_bit_OR(a:&str, b:&str) -> String {
    let mut result = String::new();
    let mut i = 0;
    for c in a.chars() {
        result.push(one_bit_OR(c, (b.chars().nth(i).unwrap())));
        i += 1;
    }
    return result;
}

fn one_bit_AND(a:char, b:char) -> char {
    if (a == '1') & (b == '1') {
        return '1';
    } else if (a == '0') & (b == '0') {
        return '1';
    } else {
        return '0';
    }
}

fn three_bit_AND(a:&str, b:&str) -> String {
    let mut result = String::new();
    let mut i = 0;
    for c in a.chars() {
        result.push(one_bit_AND(c, b.chars().nth(i).unwrap()));
        i += 1;
    }
    return result;
}

