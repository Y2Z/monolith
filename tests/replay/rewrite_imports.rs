//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::replay::rewrite_imports;
    use monolith::url::Url;

    fn rewrite(source: &str) -> String {
        let url = Url::parse("https://a.b/js/main.js").unwrap();

        rewrite_imports(source, &url).code
    }

    #[test]
    fn imports_with_bindings() {
        assert_eq!(
            rewrite("import { a } from './a.js';"),
            r#"import { a } from "https://a.b/js/a.js";"#
        );
    }

    #[test]
    fn minified_imports() {
        assert_eq!(
            rewrite(r#"import{a as b}from"../a.js";import*as c from"/c.js";"#),
            r#"import{a as b}from"https://a.b/a.js";import*as c from"https://a.b/c.js";"#
        );
    }

    #[test]
    fn imports_for_side_effects() {
        assert_eq!(
            rewrite("import './a.js';"),
            r#"import "https://a.b/js/a.js";"#
        );
    }

    #[test]
    fn exports_from_other_modules() {
        assert_eq!(
            rewrite("export * from './a.js';\nexport { b } from './b.js';"),
            "export * from \"https://a.b/js/a.js\";\nexport { b } from \"https://a.b/js/b.js\";"
        );
    }

    #[test]
    fn dynamic_imports() {
        assert_eq!(
            rewrite("load(name => import(name))"),
            r#"load(name => __monolithImport("https://a.b/js/main.js",name))"#
        );
    }

    #[test]
    fn script_url() {
        assert_eq!(
            rewrite("new URL('a.png', import.meta.url)"),
            r#"new URL('a.png', "https://a.b/js/main.js")"#
        );
    }

    #[test]
    fn location_of_the_document() {
        assert_eq!(
            rewrite("let a=window.location.pathname,b=location.hash;c(document?.location)"),
            "let a=window.__monolithLocation.pathname,b=__monolithLocation.hash;\
             c(document?.__monolithLocation)"
        );
    }

    #[test]
    fn location_as_shorthand_keeps_its_key() {
        assert_eq!(
            rewrite("f({location});const{location:a,b}=c;g({a,location})"),
            "f({location:__monolithLocation});const{location:a,b}=c;\
             g({a,location:__monolithLocation})"
        );
    }

    #[test]
    fn location_as_an_argument_or_array_item() {
        assert_eq!(
            rewrite("f(a,location,b);[a,location,b]"),
            "f(a,__monolithLocation,b);[a,__monolithLocation,b]"
        );
    }

    #[test]
    fn location_as_a_variable() {
        assert_eq!(
            rewrite("function f(location){return location.href}"),
            "function f(__monolithLocation){return __monolithLocation.href}"
        );
    }

    #[test]
    fn knows_what_is_a_module() {
        let url = Url::parse("https://a.b/main.js").unwrap();

        assert!(rewrite_imports("export const a = 1;", &url).is_module);
        assert!(rewrite_imports("export{a}", &url).is_module);
        assert!(rewrite_imports("import.meta.env", &url).is_module);
        assert!(!rewrite_imports("window.a = 1;", &url).is_module);
    }

    #[test]
    fn reports_what_gets_imported() {
        let url = Url::parse("https://a.b/main.js").unwrap();
        let imports = rewrite_imports("import './a.js';import('./b.js');import 'c';", &url).imports;

        assert_eq!(imports, vec!["https://a.b/a.js", "https://a.b/b.js"]);
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
    use monolith::replay::rewrite_imports;
    use monolith::url::Url;

    fn assert_unchanged(source: &str) {
        let url = Url::parse("https://a.b/js/main.js").unwrap();

        assert_eq!(rewrite_imports(source, &url).code, source);
    }

    #[test]
    fn names_for_import_maps_to_resolve() {
        assert_unchanged("import a from 'react';");
    }

    #[test]
    fn strings() {
        assert_unchanged(r#"const a = "import b from './b.js'";"#);
    }

    #[test]
    fn templates() {
        assert_unchanged("const a = `import b from './b.js' ${c ? `import('d')` : ''}`;");
    }

    #[test]
    fn comments() {
        assert_unchanged("// import a from './a.js'\n/* import('./b.js') */ c();");
    }

    #[test]
    fn regular_expressions() {
        assert_unchanged(r#"const a = /import("x")/.test(b), c = /[/"']/g;"#);
    }

    #[test]
    fn division_is_not_a_regular_expression() {
        let url = Url::parse("https://a.b/main.js").unwrap();
        let script = rewrite_imports("a = b / 2; import('./c.js'); d = e / 2;", &url);

        assert!(script.code.contains("__monolithImport"));
    }

    #[test]
    fn properties_named_like_keywords() {
        assert_unchanged("a.import('./b.js'); c.export = 1;");
    }

    #[test]
    fn methods_named_like_keywords() {
        assert_unchanged("class A{async import(a,b=c(1)){return a}}");
        assert_unchanged("const a={import(b){return b}};");
    }

    #[test]
    fn keys_and_methods_named_location() {
        assert_unchanged("const a={location:1,location(){return 2}};");
        assert_unchanged("class A{get location(){return 1}}");
    }

    #[test]
    fn words_containing_location() {
        assert_unchanged("const geolocation = a.relocation, $location = 1, location_ = 2;");
    }

    #[test]
    fn strings_mentioning_location() {
        assert_unchanged(r#"a("location", 'window.location', `location`);"#);
    }

    #[test]
    fn identifiers_containing_keywords() {
        assert_unchanged("reimport('./a.js'); importer('./b.js');");
    }

    #[test]
    fn code_without_any_of_it() {
        assert_unchanged("");
        assert_unchanged("(function(){return 1/2})()");
    }

    #[test]
    fn unfinished_code() {
        assert_unchanged("const a = 'never closed");
        assert_unchanged("import");
    }
}
