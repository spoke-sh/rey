import * as stylex from "@stylexjs/stylex";
import type { StyleXStyles } from "@stylexjs/stylex";

type CompiledStyle =
  Readonly<Record<string, unknown>> | false | null | undefined;

const elementStyles = stylex.create({
  borderBox: {
    boxSizing: "border-box",
    fontFamily: "inherit",
    WebkitTapHighlightColor: "transparent",
  },
});

export function className(...styles: ReadonlyArray<CompiledStyle>) {
  return stylex.props(
    elementStyles.borderBox,
    ...(styles as ReadonlyArray<StyleXStyles>),
  ).className;
}

export const globalStyles = stylex.create({
  html: {
    backgroundColor: "#d8d9d4",
    color: "#181b1b",
    fontFamily: 'Inter, "Helvetica Neue", Arial, sans-serif',
    fontSynthesis: "none",
    minWidth: 320,
    scrollBehavior: {
      default: "smooth",
      "@media (prefers-reduced-motion: reduce)": "auto",
    },
    textRendering: "geometricPrecision",
  },
  body: {
    margin: 0,
    minHeight: "100vh",
  },
  root: {
    minHeight: "100vh",
  },
});
