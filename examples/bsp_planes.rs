use gsparser::bsp::BspReader;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let path = args.get(0).expect("Expected import path!");

    let file_bytes = std::fs::read(path).expect("Failed to open file!");
    let reader = BspReader::read(file_bytes);

    let planes = reader.read_planes();
    for plane in planes {
        println!("{:?}", plane);
    }
}
