//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {

    #[test]
    #[cfg(not(feature = "browser"))]
    fn says_when_there_is_no_browser_support() {
        let mut cmd = crate::cli::monolith();
        let output = cmd.args(["-B", "https://localhost/"]).output().unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("no browser support"));
    }

    #[test]
    #[cfg(feature = "browser")]
    fn does_not_accept_stdin() {
        let mut cmd = crate::cli::monolith();
        let assert = cmd.args(["-B", "-"]).assert();

        assert.failure().code(1);
    }

    #[test]
    #[cfg(feature = "browser")]
    fn reports_invalid_browser_path() {
        let mut cmd = crate::cli::monolith();
        let assert = cmd
            .env("MONOLITH_BROWSER", "/there/is/no/such/browser")
            .args(["-B", "https://localhost/"])
            .assert();

        assert.failure().code(1);
    }
}
