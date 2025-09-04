// @react-jsx

function Comp({ propName }) {
  return (
    <div>
      {propName}
    </div>
  );
}

export function main(aaa) {
  return (
    <Comp propName={aaa}>
    </Comp>
  );
}
