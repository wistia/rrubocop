refine Foo do
  include Bar
  ^ Lint/RefinementImportMethods: Use `import_methods` instead of `include` because it is deprecated in Ruby 3.1.
end
