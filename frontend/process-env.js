import { promises as fs } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const envFile = join(__dirname, '.env');
const newEnvFile = join(__dirname, '.env');

async function processEnvFile() {
	try {
		const data = await fs.readFile(envFile, 'utf8');

		const lines = data.split('\n');
		const publicLines = lines.map((line) => {
			if (line.startsWith('CANISTER_ID') || line.startsWith('DFX_')) {
				return `VITE_${line}`;
			}
			return line;
		});

		await fs.writeFile(newEnvFile, publicLines.join('\n'));
		console.log('.env file modified with PUBLIC_ prefixes');
	} catch (err) {
		console.error('Error processing .env file:', err);
	}
}

processEnvFile();
