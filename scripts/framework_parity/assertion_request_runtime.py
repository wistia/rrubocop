"""Authored real Rails assertion/request alias programs and custom-contract safeguards."""
def programs():
    rows=[]
    assertion_header="gem 'activesupport','8.1.3.1'\ngem 'minitest','6.0.6'\nrequire 'active_support'\nrequire 'active_support/test_case'\n"
    for call in ("refute false", "refute nil", "refute_empty [1]", "refute_equal 1,2", "refute_in_delta 1.0,2.0,0.01", "refute_in_epsilon 1.0,2.0,0.01", "refute_includes [1],2", "refute_instance_of String,1", "refute_kind_of String,1", "refute_nil 1", "refute_operator 1,:>,2", "refute_predicate [],:any?", "refute_respond_to 1,:missing", "refute_same Object.new,Object.new", "refute_match /a/,'b'", "refute true", "refute true,'custom'"):
        source=assertion_header+"class Probe < ActiveSupport::TestCase\ndef probe\nbegin\n"+call+"\nrescue Minitest::Assertion=>error\np error.message\nend\np assertions\nend\nend\nProbe.new('unused').probe\n"
        rows.append(("Rails/RefuteMethods",source))
        if call.startswith('refute '):
            rows.append(("Rails/RefuteMethods",source.replace(call,call.replace('refute','assert_not',1)),dict(EnforcedStyle='refute')))
    rows.append(("Rails/RefuteMethods","def refute(*); :original; end\ndef assert_not(*); :different; end\np refute(false)\n"))
    request_header="gem 'actionpack','8.1.3.1'\nrequire 'action_dispatch'\n"
    for style,accessor in (("referer","referrer"),("referrer","referer")):
        for getter in ('request','request()'):
            source=request_header+"def request; ActionDispatch::Request.new('HTTP_REFERER'=>'good'); end\np "+getter+'.'+accessor+"\n"
            rows.append(("Rails/RequestReferer",source,dict(EnforcedStyle=style)))
        source=request_header+"request=ActionDispatch::Request.new('HTTP_REFERER'=>'bad')\ndef request; ActionDispatch::Request.new('HTTP_REFERER'=>'good'); end\np request()."+accessor+"\n"
        rows.append(("Rails/RequestReferer",source,dict(EnforcedStyle=style)))
    rows.append(("Rails/RequestReferer","class Custom\ndef referer; :different; end\ndef referrer; :original; end\nend\ndef request; Custom.new; end\np request.referrer\n"))
    rows.append(("Rails/RequestReferer",request_header+"def request; ActionDispatch::Request.new('HTTP_REFERER'=>'good'); end\np request.\nreferrer\np __LINE__\n"))
    return rows
