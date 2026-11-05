//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::CapturedResponse;

    fn response(method: &str, status: u64) -> CapturedResponse {
        CapturedResponse {
            method: method.to_string(),
            status,
            ..Default::default()
        }
    }

    #[test]
    fn complete_response() {
        assert!(response("GET", 200).is_usable());
    }

    #[test]
    fn not_modified() {
        assert!(response("GET", 304).is_usable());
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
    use monolith::browser::CapturedResponse;

    fn response(method: &str, status: u64) -> CapturedResponse {
        CapturedResponse {
            method: method.to_string(),
            status,
            ..Default::default()
        }
    }

    #[test]
    fn post_request() {
        assert!(!response("POST", 200).is_usable());
    }

    #[test]
    fn partial_response() {
        assert!(!response("GET", 206).is_usable());
    }

    #[test]
    fn error_page() {
        assert!(!response("GET", 404).is_usable());
    }
}
