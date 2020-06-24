//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use crate::url;

    #[test]
    fn removes_fragment() {
        assert_eq!(
            url::clean_url("https://somewhere.com/font.eot#iefix"),
            "https://somewhere.com/font.eot"
        );
    }

    #[test]
    fn removes_empty_fragment() {
        assert_eq!(
            url::clean_url("https://somewhere.com/font.eot#"),
            "https://somewhere.com/font.eot"
        );
    }

    #[test]
    fn removes_empty_query_and_empty_fragment() {
        assert_eq!(
            url::clean_url("https://somewhere.com/font.eot?#"),
            "https://somewhere.com/font.eot"
        );
    }

    #[test]
    fn removes_empty_query_amp_and_empty_fragment() {
        assert_eq!(
            url::clean_url("https://somewhere.com/font.eot?a=b&#"),
            "https://somewhere.com/font.eot?a=b"
        );
    }

    #[test]
    fn keeps_credentials() {
        assert_eq!(
            url::clean_url("https://cookie:monster@gibson.internet/"),
            "https://cookie:monster@gibson.internet/"
        );
    }
}
