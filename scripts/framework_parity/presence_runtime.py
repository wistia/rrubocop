"""Authored programs expose repeated receivers and precedence instead of executing fixtures."""
def programs():
    header="gem 'activesupport', '8.1.3.1'\nrequire 'active_support/core_ext/object/blank'\n"
    bodies=[
        "value='x'\np(value.present? ? value : nil)",
        "value=' '\np(value.blank? ? :fallback : value)",
        "value=nil\n$events=[]\ndef other; $events << :fallback; :other; end\np(value.present? ? value : other())\np $events",
        "$events=[]\ndef value; $events << :value; 'x'; end\np(value.present? ? value : nil)\np $events",
        "$calls=0\ndef value; $calls+=1; $calls==1 ? 'first' : 'second'; end\np(value.present? ? value : nil)\np $calls",
        "class Custom\ndef presence; :different; end\nend\nvalue=Custom.new\nresult=value.present? ? value : nil\np result.class",
        "class Custom\ndef blank?; true; end\ndef present?; true; end\nend\nvalue=Custom.new\nresult=value.blank? ? :fallback : value\np result==:fallback",
        "value=nil\nklass=Class.new do\ndefine_method(:empty?) { value=:changed; false }\nend\nvalue=klass.new\nresult=value.present? ? value : nil\np result==:changed",
        "value='x'\n$events=[]\ndef other; $events << :other; true; end\nresult=if value.present?\nvalue\nelse\nfalse and other()\nend\np [result,$events]",
        "value=nil\ndef value(*); :called; end\nbegin\nresult=if value.present?\nvalue\nelse\nvalue<1\nend\np [:result,result]\nrescue NoMethodError=>error\np [:error,error.name]\nend",
        "value=nil\nresult=if value.present?\nvalue\nelse\n:fallback\nend\np result\np __LINE__",
        "value='x'\nresult=if value.present?\n# keep\nvalue\nelse\nnil\nend\np result\np __LINE__",
        "value=false\np(value.present? ? value : :fallback)",
        "value=0\np(value.present? ? value : nil)",
        "value=' '\np(!value.present? ? :fallback : value)",
        "value='x'\np(!value.blank? ? value : :fallback)",
        "value=nil\n$events=[]\ndef other(input); $events << input; :fallback; end\nresult=if value.present?\nvalue\nelse\nother 1\nend\np [result,$events]",
        "value='x'\nresult=unless value.present?\n:fallback\nelse\nvalue\nend\np result",
        "value='x'\np('x'.present? ? 'x' : nil)",
    ]
    bodies.append("value='x'\n$events=[]\ndef finished?; $events << :condition; true; end\ndef work; $events << :work; :work; end\nresult=if value.present?\nvalue\nelse\nwork until finished?\nend\np [result,$events]")
    return [("Rails/Presence",header+body+"\n") for body in bodies]
