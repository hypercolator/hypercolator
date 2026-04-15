const tag = "[hypercolator-github]";

export function log(msg: string): void {
  console.log(`${tag} ${msg}`);
}

export function warn(msg: string): void {
  console.warn(`${tag} WARN ${msg}`);
}

export function error(msg: string): void {
  console.error(`${tag} ERROR ${msg}`);
}
