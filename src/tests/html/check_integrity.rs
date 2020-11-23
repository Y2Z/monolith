//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::html;

    #[test]
    fn empty_input_sha256() {
        assert!(html::check_integrity(
            "".as_bytes(),
            "sha256-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
        ));
    }

    #[test]
    fn sha256() {
        assert!(html::check_integrity(
            "abcdef0123456789".as_bytes(),
            "sha256-9EWAHgy4mSYsm54hmDaIDXPKLRsLnBX7lZyQ6xISNOM="
        ));
    }

    #[test]
    fn sha384() {
        assert!(html::check_integrity(
            "abcdef0123456789".as_bytes(),
            "sha384-gc9l7omltke8C33bedgh15E12M7RrAQa5t63Yb8APlpe7ZhiqV23+oqiulSJl3Kw"
        ));
    }

    #[test]
    fn sha512() {
        assert!(html::check_integrity(
            "abcdef0123456789".as_bytes(),
            "sha512-zG5B88cYMqcdiMi9gz0XkOFYw2BpjeYdn5V6+oFrMgSNjRpqL7EF8JEwl17ztZbK3N7I/tTwp3kxQbN1RgFBww=="
        ));
    }
}

//  ███████╗ █████╗ ██╗██╗     ██╗███╗   ██╗ ██████╗
//  ██╔════╝██╔══██╗██║██║     ██║████╗  ██║██╔════╝
//  █████╗  ███████║██║██║     ██║██╔██╗ ██║██║  ███╗
//  ██╔══╝  ██╔══██║██║██║     ██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║██║███████╗██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚═╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use crate::html;

    #[test]
    fn empty_hash() {
        assert!(!html::check_integrity("abcdef0123456789".as_bytes(), ""));
    }

    #[test]
    fn empty_input_empty_hash() {
        assert!(!html::check_integrity("".as_bytes(), ""));
    }

    #[test]
    fn sha256() {
        assert!(!html::check_integrity(
            "abcdef0123456789".as_bytes(),
            "sha256-badhash"
        ));
    }

    #[test]
    fn sha384() {
        assert!(!html::check_integrity(
            "abcdef0123456789".as_bytes(),
            "sha384-badhash"
        ));
    }

    #[test]
    fn sha512() {
        assert!(!html::check_integrity(
            "abcdef0123456789".as_bytes(),
            "sha512-badhash"
        ));
    }
}
