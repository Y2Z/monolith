//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::replay::insert_replay_markup;

    #[test]
    fn goes_right_after_opening_head_tag() {
        let document = b"<!-- x --><html><head lang=\"en\"><title>t</title></head></html>".to_vec();

        assert_eq!(
            insert_replay_markup(document, "<m>"),
            b"<!-- x --><html><head lang=\"en\"><m><title>t</title></head></html>".to_vec()
        );
    }

    #[test]
    fn leaves_documents_without_head_alone() {
        assert_eq!(
            insert_replay_markup(b"text".to_vec(), "<m>"),
            b"text".to_vec()
        );
    }
}
