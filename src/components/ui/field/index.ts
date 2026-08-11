import { cva, type VariantProps } from "class-variance-authority"

export { default as Field } from "./Field.vue"
export { default as FieldContent } from "./FieldContent.vue"
export { default as FieldDescription } from "./FieldDescription.vue"
export { default as FieldError } from "./FieldError.vue"
export { default as FieldGroup } from "./FieldGroup.vue"
export { default as FieldLabel } from "./FieldLabel.vue"
export { default as FieldLegend } from "./FieldLegend.vue"
export { default as FieldSeparator } from "./FieldSeparator.vue"
export { default as FieldSet } from "./FieldSet.vue"

export const fieldVariants = cva("flex w-full flex-col gap-2", {
  variants: {
    orientation: {
      vertical: "flex-col",
      horizontal: "flex-row items-center gap-2",
    },
  },
  defaultVariants: {
    orientation: "vertical",
  },
})

export type FieldVariants = VariantProps<typeof fieldVariants>
