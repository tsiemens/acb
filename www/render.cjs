const ejs = require('ejs');
const fs = require('fs');
const path = require('path');

function render(templatePathLocal, outputPathLocal, data) {
  const templatePath = path.join(__dirname, templatePathLocal);
  const outputPath = path.join(__dirname, outputPathLocal);

  ejs.renderFile(templatePath, data, {}, (err, str) => {
    if (err) throw err;

    fs.mkdirSync(path.dirname(outputPath), { recursive: true });
    fs.writeFileSync(outputPath, str);
    console.log(`✅ ${templatePathLocal} -> ${outputPathLocal}`);
  });
}

render(
  path.join('templates', 'index.ejs'),
  path.join('dist', 'index.html'),
  {}
);