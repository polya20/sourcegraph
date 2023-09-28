(call_expression
    function: (identifier) @identifier
    arguments: (_) @range
)
(call_expression
    function:
        (_ (property_identifier) @identifier)
    arguments: (_) @range
)

(function_declaration
    name: (_) @name
    (formal_parameters
        (_
            (type_annotation
                [
                    (type_identifier) @related
                    (nested_type_identifier
                        (type_identifier) @related
                    )
                ]
            )
        )
    )
)
