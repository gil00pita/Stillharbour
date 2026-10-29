import { run } from './process.mjs';
run('cargo', ['test', '-p', 'stillharbour-core', '--test', 'media_process', '--', '--nocapture']);
