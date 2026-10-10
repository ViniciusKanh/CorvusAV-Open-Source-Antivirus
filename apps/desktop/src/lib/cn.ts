/** Concatena classes condicionalmente, ignorando valores falsy.
 * Substitui a dependência `clsx` para esta necessidade pontual. */
export function cn(...classes: Array<string | false | null | undefined>): string {
  return classes.filter(Boolean).join(' ')
}
