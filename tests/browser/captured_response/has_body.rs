//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::browser::CapturedResponse;

    fn response(status: u64) -> CapturedResponse {
        CapturedResponse {
            status,
            ..Default::default()
        }
    }

    #[test]
    fn regular_responses() {
        assert!(response(200).has_body());
        assert!(response(404).has_body());
    }

    #[test]
    fn not_modified() {
        assert!(response(304).has_body());
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

    fn response(status: u64) -> CapturedResponse {
        CapturedResponse {
            status,
            ..Default::default()
        }
    }

    #[test]
    fn redirect() {
        assert!(!response(302).has_body());
    }

    #[test]
    fn partial_response() {
        assert!(!response(206).has_body());
    }

    #[test]
    fn never_completed() {
        assert!(!response(0).has_body());
    }
}
