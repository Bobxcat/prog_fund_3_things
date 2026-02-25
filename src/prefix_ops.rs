//! Chapter 5

pub fn start() {
    for i in ["+*ab-cd", "+*35-65"] {
        println!(
            "`{i}` -> {:?},{},{:?}",
            end_pre(i, 0),
            is_prefix(i),
            eval_prefix(i)
        );
    }
}

/// Fidns the end of a prefix expression, if it exists
pub fn end_pre(str_exp: &str, start: usize) -> Option<usize> {
    let ch = str_exp.chars().nth(start)?;
    if ch.is_alphanumeric() {
        Some(start)
    } else if ['+', '-', '*', '/'].contains(&ch) {
        let first_operand_end = end_pre(str_exp, start + 1)?; // First operand
        end_pre(str_exp, first_operand_end + 1) // Second operand
    } else {
        None
    }
}

pub fn is_prefix(str_exp: &str) -> bool {
    let last_char = end_pre(str_exp, 0);
    last_char.is_some_and(|last_char| last_char == str_exp.len() - 1)
}

pub fn eval_prefix(str_exp: &str) -> Option<f64> {
    if str_exp.len() == 1 {
        return str_exp.parse::<f64>().ok();
    }

    let op = str_exp.chars().nth(0)?;

    let first_operand_end = end_pre(str_exp, 1)?;
    let first_operand = eval_prefix(&str_exp[1..=first_operand_end])?;

    let second_operand_end = end_pre(str_exp, first_operand_end + 1)?;
    let second_operand = eval_prefix(&str_exp[first_operand_end + 1..=second_operand_end])?;

    let (a, b) = (first_operand, second_operand);

    Some(match op {
        '+' => a + b,
        '-' => a - b,
        '*' => a * b,
        '/' => a / b,
        _ => return None,
    })
}
