use minijinja::Environment;

fn table(elements: Vec<String>, chunk_size: usize) -> String {
    let mut output: Vec<&str> = Vec::new();
    output.push("[table=noheader]");

    for chunk in elements.chunks(chunk_size) {
        output.push("[tr]");
        for element in chunk {
            output.push("[td]");
            output.push(element);
            output.push("[/td]");
        }
        output.push("[/tr]");
    }

    output.push("[/table]");
    output.into_iter().collect()
}

fn nation_table(nations: Vec<String>, chunk_size: usize) -> String {
    let mut output: Vec<&str> = Vec::new();
    output.push("[table=noheader]");

    for chunk in nations.chunks(chunk_size) {
        output.push("[tr]");
        for nation in chunk {
            output.push("[td][nation]");
            output.push(nation);
            output.push("[/nation][/td]");
        }
        output.push("[/tr]");
    }

    output.push("[/table]");
    output.into_iter().collect()
}

pub fn register_filters(env: &mut Environment<'_>) {
    env.add_filter("table", table);
    env.add_filter("nation_table", nation_table);
}