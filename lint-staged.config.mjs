export default {
  // These are simple, as they can just run prettier directly on any number of files.
  "(*.md|*.toml|*.json|*.yaml|*.yml|*.css|*.js|*.mjs|*.ts)": ["prettier --write"],

  // Unfortunately `clippy-driver` can only accept one file at a time, so we have to create an
  // individual clippy task for each changed file.
  "*.rs": async (stagedFiles) =>
    [`rustfmt --unstable-features --edition 2021 ${stagedFiles.join(" ")}`].flat(),
};
