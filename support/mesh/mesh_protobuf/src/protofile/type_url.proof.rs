// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

verus! {

impl TypeUrl<'_> {
    /// Exact matching, including rejection of names containing a dot.
    pub closed spec fn accepts(&self, input: Seq<char>) -> bool {
        without_https(input) == "type.googleapis.com/"@ + self.package@ + seq!['.'] + self.name@
            && !self.name@.contains('.')
    }
}

pub proof fn lemma_type_url_spellings(expected: &TypeUrl<'_>)
    requires !expected.name@.contains('.'),
    ensures
        expected.accepts("type.googleapis.com/"@ + expected.package@ + seq!['.'] + expected.name@),
        expected.accepts("https://type.googleapis.com/"@ + expected.package@ + seq!['.'] + expected.name@),
{
    reveal_strlit("https://");
    reveal_strlit("type.googleapis.com/");
    reveal_strlit("https://type.googleapis.com/");
    let canonical = "type.googleapis.com/"@ + expected.package@ + seq!['.'] + expected.name@;
    let secure = "https://type.googleapis.com/"@ + expected.package@ + seq!['.'] + expected.name@;
    assert(!"https://"@.is_prefix_of(canonical)) by {
        assert(canonical[0] == 't');
        assert(canonical.subrange(0, "https://"@.len() as int)[0] == 't');
    }
    assert(secure =~= "https://"@ + canonical);
    assert(secure.take("https://"@.len() as int) =~= "https://"@);
    assert(secure.skip("https://"@.len() as int) =~= canonical);
}

proof fn lemma_url_shape(s: Seq<char>, package: Seq<char>, name: Seq<char>)
    ensures
        "type.googleapis.com/"@.is_prefix_of(s) ==>
            s == "type.googleapis.com/"@ + s.skip("type.googleapis.com/"@.len() as int),
        s == "type.googleapis.com/"@ + package + seq!['.'] + name ==>
            "type.googleapis.com/"@.is_prefix_of(s)
            && s.skip("type.googleapis.com/"@.len() as int) == package + seq!['.'] + name
            && s.skip("type.googleapis.com/"@.len() as int).contains('.'),
{
    let prefix = "type.googleapis.com/"@;
    let suffix = package + seq!['.'] + name;
    if prefix.is_prefix_of(s) {
        s.lemma_split_at(prefix.len() as int);
    }
    if s == prefix + package + seq!['.'] + name {
        assert(s =~= prefix + suffix);
        assert(s.take(prefix.len() as int) =~= prefix);
        assert(s.skip(prefix.len() as int) =~= suffix);
        assert(suffix[package.len() as int] == '.');
    }
}

proof fn lemma_last_dot_unique(
    left: Seq<char>, right: Seq<char>, package: Seq<char>, name: Seq<char>,
)
    requires !right.contains('.'),
    ensures
        (left + seq!['.'] + right == package + seq!['.'] + name && !name.contains('.'))
            <==> (left == package && right == name),
{
    let s = left + seq!['.'] + right;
    if s == package + seq!['.'] + name && !name.contains('.') {
        assert(s.len() == left.len() + 1 + right.len());
        assert(s.len() == package.len() + 1 + name.len());
        assert(s[left.len() as int] == '.');
        assert(s[package.len() as int] == '.');
        if left.len() < package.len() {
            let i = package.len() as int - left.len() as int - 1;
            assert(0 <= i < right.len());
            assert(right[i] == s[package.len() as int]);
            assert(right.contains('.'));
        }
        if package.len() < left.len() {
            let i = left.len() as int - package.len() as int - 1;
            assert(0 <= i < name.len());
            assert(name[i] == s[left.len() as int]);
            assert(name.contains('.'));
        }
        assert(left.len() == package.len());
        assert(left =~= s.take(left.len() as int));
        assert(package =~= s.take(package.len() as int));
        assert(right =~= s.skip(left.len() as int + 1));
        assert(name =~= s.skip(package.len() as int + 1));
    }
}

proof fn lemma_parser_comparison_contract(expected: &TypeUrl<'_>, actual: &str, result: bool)
    ensures
        call_ensures(<&TypeUrl<'_> as PartialEq<&str>>::eq,
            (&expected, &actual), result) ==> result == expected.accepts(actual@),
        call_ensures(<&TypeUrl<'_> as PartialEq<&str>>::ne,
            (&expected, &actual), result) ==> result == !expected.accepts(actual@),
{
}

} // verus!
