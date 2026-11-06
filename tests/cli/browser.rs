//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod failing {
    use assert_cmd::cargo_bin_cmd;

    #[test]
    fn does_not_accept_stdin() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let assert = cmd.args(["-B", "-"]).assert();

        assert.failure().code(1);
    }

    #[test]
    fn reports_invalid_browser_path() {
        let mut cmd = cargo_bin_cmd!(env!("CARGO_PKG_NAME"));
        let assert = cmd
            .env("MONOLITH_BROWSER", "/there/is/no/such/browser")
            .args(["-B", "https://localhost/"])
            .assert();

        assert.failure().code(1);
    }
}
