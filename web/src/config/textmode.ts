export type TextmodeConfig = {
  columns: number;
  bodyWidth: number;
  textIndent: number;
  articleArtIndent: number;
  volumeArtIndent: number;
  volumeRightColumn: number;
  mobileFitBreakpoint: number;
};

export const textmodeConfig: TextmodeConfig = {
  columns: 90,
  bodyWidth: 80,
  textIndent: 4,
  articleArtIndent: 60,
  volumeArtIndent: 60,
  volumeRightColumn: 83,
  mobileFitBreakpoint: 760
};
