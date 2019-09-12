pragma solidity >=0.5.0 <0.6.0;

contract Test {

    event EventThrown(uint256 id);

    constructor() public {}

    function emitValue(uint256 id) public {
        emit EventThrown(id);
    }

    function returnValue(uint256 id) public returns (uint256) {
        emit EventThrown(id);
        return id;
    }
}