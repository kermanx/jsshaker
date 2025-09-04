// @react-jsx

function Comp({ x, y }) {
  const unused = <div b={effect()} />;

  return (
    <div>
      {x}
      {y && <div> Lots of things never rendered </div>}
    </div>
  );
}

export function main() {
  return (
    <Comp x={1}>
      <div id="1" />
      Hello
    </Comp>
  );
}
