use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref CATEGORIES: HashMap<&'static str, (u64, HashMap<&'static str, u64>)> = create_category_map();
}

pub fn parse_categories(category: &String) -> Option<(u64, u64)> {
    let Some((category_id, subcategory_id)) = category.split_once("/") else {
        eprintln!("Invalid category string '{}'!", category);
        return None;
    };

    let Some(category) = CATEGORIES.get(category_id) else {
        eprintln!("Invalid category '{}'!", category_id);
        return None;
    };

    let Some(subcategory) = category.1.get(subcategory_id) else {
        eprintln!("Invalid subcategory '{}'!", subcategory_id);
        return None;
    };
    
    let real_subcategory = (category.0 * 100) + (*subcategory);

    Some((category.0, real_subcategory))
}

fn create_category_map() -> HashMap<&'static str, (u64, HashMap<&'static str, u64>)> {
    let mut map = HashMap::new();

    map.insert("Factbook", (1, HashMap::from([
        ("Overview", 0),
        ("History", 1),
        ("Geography", 2),
        ("Culture", 3),
        ("Politics", 4),
        ("Legislation", 5),
        ("Religion", 6),
        ("Military", 7),
        ("Economy", 8),
        ("International", 9),
        ("Trivia", 10),
        ("Miscellaneous", 11),
    ])));

    map.insert("Bulletin", (3, HashMap::from([
        ("Policy", 5),
        ("News", 15),
        ("Opinion", 25),
        ("Campaign", 85),
    ])));

    map.insert("Account", (5, HashMap::from([
        ("Military", 5),
        ("Trade", 15),
        ("Sport", 25),
        ("Drama", 35),
        ("Diplomacy", 45),
        ("Science", 55),
        ("Culture", 65),
        ("Other", 95),
    ])));

    map.insert("Meta", (8, HashMap::from([
        ("Gameplay", 35),
        ("Reference", 45),
    ])));

    map
}