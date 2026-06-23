use crate::protocol::pixel_format::PixelFormat;

#[test]
fn endian_pixel_format_conversion() {
    let src_format = PixelFormat {
        bits_per_pixel: crate::protocol::pixel_format::BitsPerPixel::U32,
        depth: 24,
        big_endian: crate::protocol::primitives::Flag::No,
        true_color: crate::protocol::primitives::Flag::Yes,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };

    let dest_format = PixelFormat {
        bits_per_pixel: crate::protocol::pixel_format::BitsPerPixel::U32,
        depth: 24,
        big_endian: crate::protocol::primitives::Flag::Yes,
        true_color: crate::protocol::primitives::Flag::Yes,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };

    let data = vec![
        0xdeu8, 0xad, 0xbe, 0xef, 0xdeu8, 0xad, 0xbe, 0xef, 0xdeu8, 0xad, 0xbe, 0xef,
    ];

    print!("source :");
    data.iter().for_each(|b| print!("{:02X} ", b));
    println!();

    let converted = src_format
        .convert_data_to_pixel_format(&dest_format, &data)
        .unwrap();

    print!("dest :");
    converted.iter().for_each(|b| print!("{:0>2x} ", b));
    println!();

    assert_eq!(
        converted,
        vec![
            0x0, 0xbe, 0xad, 0xdeu8, 0x0, 0xbe, 0xad, 0xdeu8, 0x0, 0xbe, 0xad, 0xdeu8,
        ]
    )
}

#[test]
fn u32_to_u16_pixel_format_conversion() {
    let src_format = PixelFormat {
        bits_per_pixel: crate::protocol::pixel_format::BitsPerPixel::U32,
        depth: 24,
        big_endian: crate::protocol::primitives::Flag::Yes,
        true_color: crate::protocol::primitives::Flag::Yes,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };

    let dest_format = PixelFormat {
        bits_per_pixel: crate::protocol::pixel_format::BitsPerPixel::U16,
        depth: 16,
        big_endian: crate::protocol::primitives::Flag::Yes,
        true_color: crate::protocol::primitives::Flag::Yes,
        red_max: 31,
        green_max: 63,
        blue_max: 31,
        red_shift: 11,
        green_shift: 5,
        blue_shift: 0,
    };

    let data = vec![
        0x0u8, 0xff, 0x0, 0x0, 0x0, 0x0, 0xff, 0x0, 0x0, 0x0, 0x0, 0xff,
    ];

    print!("source :");
    data.iter().for_each(|b| print!("{:02X} ", b));
    println!();

    let converted = src_format
        .convert_data_to_pixel_format(&dest_format, &data)
        .unwrap();

    print!("dest :");
    converted.iter().for_each(|b| print!("{:0>2x} ", b));
    println!();

    assert_eq!(converted, vec![0xf8, 0x00, 0x07, 0xe0, 0x00, 0x1f])
}
