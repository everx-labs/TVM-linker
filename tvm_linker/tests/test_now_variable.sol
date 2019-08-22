pragma solidity ^0.5.0;

contract TestNow {

    function main() public payable {
        require(now > 0);
    }
}