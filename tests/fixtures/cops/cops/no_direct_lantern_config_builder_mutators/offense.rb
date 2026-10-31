config_builder.append_element(x)
# rrubocop-expect: 1:0 Cops/NoDirectLanternConfigBuilderMutators: Direct ConfigBuilder mutator call. Route writes through `config_builder.apply_mutation(:append_element, **payload)` so the PreWriteValidators + PostWriteCleanup pipeline runs. See VideoRemixLanternAgent::Mutations for the dispatch.
