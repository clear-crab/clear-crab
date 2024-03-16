//! Lexing, bridging to parser (which does the actual parsing) and
//! incremental reparsing.

mod reparsing;

use rowan::TextRange;

use crate::{syntax_node::GreenNode, SyntaxError, SyntaxTreeBuilder};

pub(crate) use crate::parsing::reparsing::incremental_reparse;

pub(crate) fn parse_text(text: &str) -> (GreenNode, Vec<SyntaxError>) {
    let _p = tracing::span!(tracing::Level::INFO, "parse_text").entered();
    let lexed = parser::LexedStr::new(text);
    let parser_input = lexed.to_input();
    let parser_output = parser::TopEntryPoint::SourceFile.parse(&parser_input);
    let (node, errors, _eof) = build_tree(lexed, parser_output);
    (node, errors)
}

pub(crate) fn build_tree(
    lexed: parser::LexedStr<'_>,
    parser_output: parser::Output,
) -> (GreenNode, Vec<SyntaxError>, bool) {
    let _p = tracing::span!(tracing::Level::INFO, "build_tree").entered();
    let mut builder = SyntaxTreeBuilder::default();

    let is_eof = lexed.intersperse_trivia(&parser_output, &mut |step| match step {
        parser::StrStep::Token { kind, text } => builder.token(kind, text),
        parser::StrStep::Enter { kind } => builder.start_node(kind),
        parser::StrStep::Exit => builder.finish_node(),
        parser::StrStep::Error { msg, pos } => {
            builder.error(msg.to_owned(), pos.try_into().unwrap())
        }
    });

    let (node, mut errors) = builder.finish_raw();
    for (i, err) in lexed.errors() {
        let text_range = lexed.text_range(i);
        let text_range = TextRange::new(
            text_range.start.try_into().unwrap(),
            text_range.end.try_into().unwrap(),
        );
        errors.push(SyntaxError::new(err, text_range))
    }

    (node, errors, is_eof)
}
