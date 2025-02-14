#pragma once

#include "Tool.h"
#include "Parsers.h"

namespace cs::semantic {

    struct ToolAr : public Tool {

        [[nodiscard]]
        rust::Result<SemanticPtr> recognize(const Execution &execution) const override;

    protected:
        [[nodiscard]]
        bool is_ar_call(const fs::path& program) const;

        [[nodiscard]]
        rust::Result<SemanticPtr> archiving(const Execution &execution) const;

        [[nodiscard]]
        static rust::Result<SemanticPtr> archiving(const FlagsByName &flags, const Execution &execution);

        static const FlagsByName FLAG_DEFINITION;
    };
} 