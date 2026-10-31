fooBar = 1
^^^^^^ Naming/VariableName: Use snake_case for variable names.

@BadIvar = 1
^^^^^^^^ Naming/VariableName: Use snake_case for variable names.

def pattern_reads(withdrawal)
  withdrawal => { applyTime:, info: fail_reason }
  _ = Integer(applyTime)
              ^^^^^^^^^ Naming/VariableName: Use snake_case for variable names.
  _ = applyTime.present?
      ^^^^^^^^^ Naming/VariableName: Use snake_case for variable names.
  _ = fail_reason
end

items.each { |_Foo| _Foo }
              ^^^^ Naming/VariableName: Use snake_case for variable names.
                    ^^^^ Naming/VariableName: Use snake_case for variable names.

_badName = 1
^^^^^^^^ Naming/VariableName: Use snake_case for variable names.
