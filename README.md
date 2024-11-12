1 = $sum();
2 = $pos();

<
    +({
        $left: $pos(),
        $right: $sum()
    }) := $sum();

    +({
        $left: $pos(),
        $right: 1
    }) := $pos();

    [
        {
            $case: <#inner({$left: 101, $right: $pos()}) := $result();>,
            $then: <$() := $result();>
        },
        {$case: <1 := $result();>}
    ] = $switch();
> = $loop();

$sum() := $result();
