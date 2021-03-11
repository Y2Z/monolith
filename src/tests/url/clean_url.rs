//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use reqwest::Url;

    use crate::url;

    #[test]
    fn preserve_original() {
        let u: Url = Url::parse("https://somewhere.com/font.eot#iefix").unwrap();

        url::clean_url(u.clone());

        assert_eq!(u.as_str(), "https://somewhere.com/font.eot#iefix");
    }

    #[test]
    fn removes_fragment() {
        assert_eq!(
            url::clean_url(Url::parse("https://somewhere.com/font.eot#iefix").unwrap()).as_str(),
            "https://somewhere.com/font.eot"
        );
    }

    #[test]
    fn removes_empty_fragment() {
        assert_eq!(
            url::clean_url(Url::parse("https://somewhere.com/font.eot#").unwrap()).as_str(),
            "https://somewhere.com/font.eot"
        );
    }

    #[test]
    fn removes_empty_fragment_and_keeps_empty_query() {
        assert_eq!(
            url::clean_url(Url::parse("https://somewhere.com/font.eot?#").unwrap()).as_str(),
            "https://somewhere.com/font.eot?"
        );
    }

    #[test]
    fn removesempty_fragment_and_keeps_empty_query() {
        assert_eq!(
            url::clean_url(Url::parse("https://somewhere.com/font.eot?a=b&#").unwrap()).as_str(),
            "https://somewhere.com/font.eot?a=b&"
        );
    }

    #[test]
    fn keeps_credentials() {
        assert_eq!(
            url::clean_url(Url::parse("https://cookie:monster@gibson.internet/").unwrap()).as_str(),
            "https://cookie:monster@gibson.internet/"
        );
    }
}
